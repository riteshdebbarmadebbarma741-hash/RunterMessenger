// servers/rmp-server/src/dlq.rs
use crate::error::RmpError;
use runter_database::wal::{WalEntry, WriteAheadLog};
use std::sync::Arc;

pub struct DeadLetterQueue {
    wal: Arc<WriteAheadLog>,
}

impl DeadLetterQueue {
    pub fn new(wal: Arc<WriteAheadLog>) -> Self {
        DeadLetterQueue { wal }
    }

    pub fn push(&self, entry: &WalEntry, error: &str) -> Result<(), RmpError> {
        let mut dlq_entry = WalEntry {
            index: 0,
            entry_type: runter_database::wal::WalEntryType::MessageInsert,
            queue_id: entry.queue_id.clone(),
            sequence_id: entry.sequence_id,
            message_id: entry.message_id.clone(),
            payload: format!("DLQ|{}|{}", hex::encode(&entry.message_id), error).into_bytes(),
            timestamp: runter_protocol::types::now_secs(),
            ttl: None,
            expires_at: None,
            crc: 0,
        };
        self.wal.append(&mut dlq_entry)?;
        Ok(())
    }
}