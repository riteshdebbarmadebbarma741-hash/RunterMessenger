// core/database/src/message_store.rs
use crate::Database;
use crate::error::DatabaseError;
use crate::wal::{WalEntry, WalEntryType};
use runter_protocol::types::{Message, QueueId, MessageId};
use std::sync::Arc;

pub struct MessageStore;

impl MessageStore {
    pub fn insert_batch(db: &Database, messages: &[Message]) -> Result<Vec<u64>, DatabaseError> {
        let mut entries: Vec<WalEntry> = Vec::with_capacity(messages.len());
        for msg in messages {
            let sequence_id = db.sequence.next(&msg.queue_id.0)?;
            entries.push(WalEntry {
                index: 0,
                entry_type: WalEntryType::MessageInsert,
                queue_id: msg.queue_id.0.to_vec(),
                sequence_id,
                message_id: msg.id.0.to_vec(),
                payload: msg.payload.to_vec(),
                timestamp: runter_protocol::types::now_secs(),
                ttl: msg.ttl,
                expires_at: msg.expires_at,
                crc: 0,
            });
        }
        let last_index = db.wal.append_batch(&mut entries)?;
        for entry in &entries {
            db.materializer.apply(entry)?;
            db.wal.mark_applied(&entry.index)?;
        }
        db.metrics.materialized_messages.inc_by(entries.len() as u64);
        Ok(entries.iter().map(|e| e.index).collect())
    }

    pub fn get_unacked(db: &Database, queue_id: &QueueId, limit: usize) -> Result<Vec<Message>, DatabaseError> {
        let conn = db.materializer.sqlite.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, sequence_id, timestamp, ttl, expires_at FROM messages WHERE queue_id = ?1 AND acked = 0 ORDER BY sequence_id ASC LIMIT ?2"
        )?;
        let messages = stmt.query_map(rusqlite::params![queue_id.0.to_vec(), limit as i64], |row| {
            let id_bytes: Vec<u8> = row.get(0)?;
            let sequence_id: i64 = row.get(1)?;
            let id_array: [u8; 16] = id_bytes.try_into().unwrap_or_default();
            let key = super::materializer::build_materialized_key(&queue_id.0, sequence_id);
            let payload = db.materializer.rocks.get(&key).ok().flatten().unwrap_or_default();
            Ok(Message {
                id: MessageId(id_array),
                queue_id: queue_id.clone(),
                payload: Arc::new(payload),
                timestamp: row.get(2)?,
                ttl: row.get(3)?,
                acked: false,
                expires_at: row.get(4)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(messages)
    }

    pub fn ack_batch(db: &Database, queue_id: &QueueId, message_ids: &[MessageId]) -> Result<(), DatabaseError> {
        let mut entries: Vec<WalEntry> = Vec::with_capacity(message_ids.len());
        for msg_id in message_ids {
            entries.push(WalEntry {
                index: 0,
                entry_type: WalEntryType::MessageAck,
                queue_id: queue_id.0.to_vec(),
                sequence_id: 0,
                message_id: msg_id.0.to_vec(),
                payload: vec![],
                timestamp: 0,
                ttl: None,
                expires_at: None,
                crc: 0,
            });
        }
        db.wal.append_batch(&mut entries)?;
        for entry in &entries {
            db.materializer.apply(entry)?;
            db.wal.mark_applied(&entry.index)?;
        }
        db.metrics.materialized_acks.inc_by(entries.len() as u64);
        Ok(())
    }
}