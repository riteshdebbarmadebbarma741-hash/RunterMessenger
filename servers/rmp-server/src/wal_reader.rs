// servers/rmp-server/src/wal_reader.rs
use crate::config::RmpConfig;
use crate::error::RmpError;
use runter_database::wal::{WalEntry, WriteAheadLog};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct WalReader {
    wal: Arc<WriteAheadLog>,
    watermark: AtomicU64,
    config: RmpConfig,
}

impl WalReader {
    pub fn new(wal: Arc<WriteAheadLog>, config: &RmpConfig) -> Self {
        WalReader {
            wal,
            watermark: AtomicU64::new(0),
            config: config.clone(),
        }
    }

    pub fn snapshot_watermark(&self) -> u64 {
        self.watermark.load(Ordering::Acquire)
    }

    pub fn advance_watermark(&self, new_watermark: u64) {
        self.watermark.fetch_max(new_watermark, Ordering::Release);
    }

    pub fn read_window(&self, start: u64, end: u64) -> Result<Vec<WalEntry>, RmpError> {
        let entries = self.wal.read_from(start)?;
        Ok(entries.into_iter().filter(|e| e.index <= end).collect())
    }

    pub fn read_exact(&self, start: u64, count: usize) -> Result<Vec<WalEntry>, RmpError> {
        let entries = self.wal.read_from(start)?;
        Ok(entries.into_iter().take(count).collect())
    }
}