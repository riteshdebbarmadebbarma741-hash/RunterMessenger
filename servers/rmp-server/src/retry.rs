// servers/rmp-server/src/retry.rs
use crate::config::RmpConfig;
use crate::error::RmpError;
use runter_database::wal::{WalEntry, WriteAheadLog};
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::Mutex;

pub struct RetryManager {
    wal: Arc<WriteAheadLog>,
    config: RmpConfig,
    attempts: Mutex<HashMap<Vec<u8>, u32>>,
}

impl RetryManager {
    pub fn new(wal: Arc<WriteAheadLog>, config: &RmpConfig) -> Self {
        RetryManager {
            wal,
            config: config.clone(),
            attempts: Mutex::new(HashMap::new()),
        }
    }

    pub fn schedule_retry(&self, entry: &WalEntry) -> Result<(), RmpError> {
        let mut attempts = self.attempts.lock();
        let count = attempts.entry(entry.message_id.clone()).or_insert(0);
        *count += 1;
        if *count > self.config.retry_max_attempts {
            return Err(RmpError::RetryExhausted(hex::encode(&entry.message_id)));
        }
        let delay_ms = self.config.retry_base_delay_ms * 2u64.pow(*count - 1);
        let delay_ms = delay_ms.min(self.config.retry_max_delay_ms);
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        let mut retry_entry = entry.clone();
        retry_entry.index = 0;
        retry_entry.crc = 0;
        self.wal.append(&mut retry_entry)?;
        Ok(())
    }
}