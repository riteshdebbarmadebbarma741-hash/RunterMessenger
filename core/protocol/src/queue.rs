// core/protocol/src/queue.rs
use crate::constants::{MAX_MESSAGES_PER_QUEUE, MAX_SUBSCRIBERS_PER_QUEUE};
use crate::error::ProtocolError;
use crate::types::{Message, QueueConfig, QueueId, MessageId};
use std::collections::{VecDeque, HashSet};

pub struct Queue {
    pub id: QueueId,
    pub config: QueueConfig,
    messages: VecDeque<Message>,
    next_delivery_index: usize,
    pub subscribers: Vec<String>,
    pub created_at: u64,
    pub name: String,
}

impl Queue {
    pub fn new(id: QueueId, config: QueueConfig) -> Self {
        Queue {
            id,
            name: config.name.clone(),
            config,
            messages: VecDeque::new(),
            next_delivery_index: 0,
            subscribers: Vec::new(),
            created_at: crate::types::now_secs(),
        }
    }

    pub fn push(&mut self, mut message: Message) -> Result<(), ProtocolError> {
        self.cleanup_expired();
        if self.messages.len() as u64 >= self.config.max_size.min(MAX_MESSAGES_PER_QUEUE) {
            self.cleanup_acked();
        }
        if self.messages.len() as u64 >= self.config.max_size.min(MAX_MESSAGES_PER_QUEUE) {
            return Err(ProtocolError::QueueError("Queue full".into()));
        }
        message.timestamp = crate::types::now_secs();
        self.messages.push_back(message);
        Ok(())
    }

    pub fn pull(&mut self, since_id: Option<&MessageId>, batch_size: usize) -> Vec<Message> {
        self.cleanup_expired();
        let batch_size = batch_size.min(crate::constants::MAX_BATCH_SIZE);
        if self.next_delivery_index > self.messages.len() {
            self.next_delivery_index = self.messages.len();
        }
        let start_index = match since_id {
            Some(since) => {
                self.messages.iter()
                    .position(|m| m.id == *since)
                    .map(|pos| pos + 1)
                    .unwrap_or(self.next_delivery_index)
            }
            None => self.next_delivery_index,
        };
        let start_index = start_index.min(self.messages.len());
        let results: Vec<Message> = self.messages.iter()
            .skip(start_index)
            .filter(|m| !m.acked && !m.is_expired())
            .take(batch_size)
            .cloned()
            .collect();
        if !results.is_empty() {
            if let Some(last) = results.last() {
                if let Some(pos) = self.messages.iter().position(|m| m.id == last.id) {
                    self.next_delivery_index = pos + 1;
                }
            }
        }
        results
    }

    pub fn ack(&mut self, message_ids: &[MessageId]) {
        let id_set: HashSet<&[u8]> = message_ids.iter().map(|id| &id.0[..]).collect();
        for msg in &mut self.messages {
            if id_set.contains(&msg.id.0[..]) {
                msg.acked = true;
            }
        }
    }

    pub fn cleanup_acked(&mut self) {
        let old_len = self.messages.len();
        self.messages.retain(|m| !m.acked);
        let removed = old_len.saturating_sub(self.messages.len());
        self.next_delivery_index = self.next_delivery_index.saturating_sub(removed);
        self.next_delivery_index = self.next_delivery_index.min(self.messages.len());
    }

    pub fn cleanup_expired(&mut self) {
        let old_len = self.messages.len();
        self.messages.retain(|m| !m.is_expired());
        let removed = old_len.saturating_sub(self.messages.len());
        self.next_delivery_index = self.next_delivery_index.saturating_sub(removed);
        self.next_delivery_index = self.next_delivery_index.min(self.messages.len());
    }

    pub fn add_subscriber(&mut self, subscriber_id: String) -> Result<(), ProtocolError> {
        if self.subscribers.len() >= MAX_SUBSCRIBERS_PER_QUEUE {
            return Err(ProtocolError::TooManySubscribers);
        }
        if !self.subscribers.contains(&subscriber_id) {
            self.subscribers.push(subscriber_id);
        }
        Ok(())
    }

    pub fn remove_subscriber(&mut self, subscriber_id: &str) {
        self.subscribers.retain(|s| s != subscriber_id);
    }

    pub fn is_expired(&self) -> bool {
        self.config.ttl_seconds
            .map_or(false, |ttl| crate::types::now_secs() > self.created_at + ttl)
    }

    pub fn is_empty(&self) -> bool {
        self.messages.iter().all(|m| m.acked)
    }

    pub fn pending_count(&self) -> usize {
        self.messages.iter().filter(|m| !m.acked && !m.is_expired()).count()
    }
}