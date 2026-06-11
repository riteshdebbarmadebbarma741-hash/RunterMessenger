// core/protocol/src/server.rs
use crate::constants::{MAX_PAYLOAD_SIZE, MAX_QUEUES_PER_CONNECTION, BACKPRESSURE_THRESHOLD};
use crate::error::ProtocolError;
use crate::types::*;
use crate::queue::Queue;
use crate::session::Session;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::interval;
use tokio::sync::Notify;
use std::time::Duration;

pub struct RmpServer {
    queues: Arc<RwLock<HashMap<String, Queue>>>,
    queue_names: Arc<RwLock<HashMap<String, String>>>,
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    shutdown: Arc<Notify>,
}

impl RmpServer {
    pub fn new() -> Self {
        let server = RmpServer {
            queues: Arc::new(RwLock::new(HashMap::new())),
            queue_names: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            shutdown: Arc::new(Notify::new()),
        };
        server.start_cleanup_task();
        server
    }

    fn start_cleanup_task(&self) {
        let queues = self.queues.clone();
        let sessions = self.sessions.clone();
        let shutdown = self.shutdown.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(crate::constants::CLEANUP_INTERVAL_SECS));
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let mut queues = queues.write().await;
                        queues.retain(|_, queue| {
                            queue.cleanup_acked();
                            queue.cleanup_expired();
                            !queue.is_expired()
                        });
                        let mut sessions = sessions.write().await;
                        sessions.retain(|_, session| !session.is_expired());
                    }
                    _ = shutdown.notified() => { break; }
                }
            }
        });
    }

    pub fn shutdown(&self) { self.shutdown.notify_one(); }

    pub async fn handle_frame(&self, frame: Frame, connection_id: &str) -> Result<Frame, ProtocolError> {
        match frame.command {
            Command::Ping => {
                let mut sessions = self.sessions.write().await;
                let session = sessions.entry(connection_id.to_string())
                    .or_insert_with(|| Session::new(vec![]));
                crate::protocol::server_handshake(session, &frame).await
            }
            Command::Auth => {
                let mut sessions = self.sessions.write().await;
                let session = sessions.entry(connection_id.to_string())
                    .or_insert_with(|| Session::new(vec![]));
                session.check_and_store_nonce(&frame.nonce)?;
                crate::protocol::server_authenticate(session, &frame).await
            }
            _ => {
                let sessions = self.sessions.read().await;
                let session = sessions.get(connection_id)
                    .ok_or(ProtocolError::AuthenticationFailed("No session".into()))?;
                if !session.authenticated {
                    return Err(ProtocolError::AuthenticationFailed("Not authenticated".into()));
                }
                drop(sessions);
                let mut sessions = self.sessions.write().await;
                let session = sessions.get_mut(connection_id)
                    .ok_or(ProtocolError::AuthenticationFailed("Session lost".into()))?;
                session.check_and_store_nonce(&frame.nonce)?;
                session.touch();
                drop(sessions);
                self.handle_authenticated_frame(frame, connection_id).await
            }
        }
    }

    async fn handle_authenticated_frame(&self, frame: Frame, connection_id: &str) -> Result<Frame, ProtocolError> {
        match frame.command {
            Command::CreateQueue => {
                let mut sessions = self.sessions.write().await;
                let session = sessions.get_mut(connection_id)
                    .ok_or(ProtocolError::AuthenticationFailed("Session lost".into()))?;
                session.check_queue_limit()?;
                let config: QueueConfig = bincode::deserialize(&frame.payload)
                    .map_err(|e| ProtocolError::DecodingFailed(e.to_string()))?;
                crate::protocol::validate_queue_name(&config.name)?;
                let mut queue_names = self.queue_names.write().await;
                if queue_names.contains_key(&config.name) {
                    return Err(ProtocolError::QueueNameExists);
                }
                let total_queues = self.queues.read().await.len();
                if total_queues >= MAX_QUEUES_PER_CONNECTION * 100 {
                    return Err(ProtocolError::TooManyQueues);
                }
                let queue_id = QueueId::generate();
                let queue = Queue::new(queue_id.clone(), config.clone());
                queue_names.insert(config.name.clone(), hex::encode(queue_id.0));
                self.queues.write().await.insert(hex::encode(queue_id.0), queue);
                session.increment_queues();
                Ok(Frame::response_to(&frame, Command::CreateQueue, bincode::serialize(&queue_id)
                    .map_err(|e| ProtocolError::EncodingFailed(e.to_string()))?))
            }
            Command::SendMessage => {
                let msg: Message = bincode::deserialize(&frame.payload)
                    .map_err(|e| ProtocolError::DecodingFailed(e.to_string()))?;
                if msg.payload.len() > MAX_PAYLOAD_SIZE {
                    return Err(ProtocolError::PayloadTooLarge(msg.payload.len()));
                }
                if msg.is_expired() {
                    return Err(ProtocolError::MessageExpired);
                }
                let pending: usize = self.queues.read().await.values().map(|q| q.pending_count()).sum();
                if pending > BACKPRESSURE_THRESHOLD {
                    return Err(ProtocolError::Backpressure);
                }
                let key = hex::encode(msg.queue_id.0);
                let mut queues = self.queues.write().await;
                match queues.get_mut(&key) {
                    Some(queue) => queue.push(msg)?,
                    None => return Err(ProtocolError::QueueNotFound),
                }
                Ok(Frame::response_to(&frame, Command::SendMessage, vec![]))
            }
            Command::ReceiveMessage => {
                let req: SubscribeRequest = bincode::deserialize(&frame.payload)
                    .map_err(|e| ProtocolError::DecodingFailed(e.to_string()))?;
                let key = hex::encode(req.queue_id.0);
                let mut queues = self.queues.write().await;
                let messages = match queues.get_mut(&key) {
                    Some(queue) => queue.pull(req.since_message_id.as_ref(), req.batch_size),
                    None => return Err(ProtocolError::QueueNotFound),
                };
                Ok(Frame::response_to(&frame, Command::ReceiveMessage, bincode::serialize(&messages)
                    .map_err(|e| ProtocolError::EncodingFailed(e.to_string()))?))
            }
            Command::AckMessage => {
                let req: AckRequest = bincode::deserialize(&frame.payload)
                    .map_err(|e| ProtocolError::DecodingFailed(e.to_string()))?;
                let key = hex::encode(req.queue_id.0);
                let mut queues = self.queues.write().await;
                match queues.get_mut(&key) {
                    Some(queue) => queue.ack(&req.message_ids),
                    None => return Err(ProtocolError::QueueNotFound),
                }
                Ok(Frame::response_to(&frame, Command::AckMessage, vec![]))
            }
            Command::DeleteQueue => {
                let queue_id: QueueId = bincode::deserialize(&frame.payload)
                    .map_err(|e| ProtocolError::DecodingFailed(e.to_string()))?;
                let key = hex::encode(queue_id.0);
                let mut queues = self.queues.write().await;
                match queues.remove(&key) {
                    Some(queue) => {
                        let mut queue_names = self.queue_names.write().await;
                        queue_names.remove(&queue.name);
                        let mut sessions = self.sessions.write().await;
                        if let Some(session) = sessions.get_mut(connection_id) {
                            session.decrement_queues();
                        }
                        Ok(Frame::response_to(&frame, Command::DeleteQueue, vec![]))
                    }
                    None => Err(ProtocolError::QueueNotFound),
                }
            }
            _ => Err(ProtocolError::InvalidFrame("Unknown command".into())),
        }
    }

    pub async fn stats(&self) -> (usize, usize, usize) {
        let queues = self.queues.read().await;
        let sessions = self.sessions.read().await;
        (queues.len(), sessions.len(), queues.values().map(|q| q.pending_count()).sum())
    }
}