// core/database/src/sequence.rs
use crate::config::DatabaseConfig;
use crate::error::DatabaseError;
use rocksdb::{Options, DB};
use std::sync::Arc;

pub struct SequenceGenerator {
    db: DB,
    cache: parking_lot::Mutex<std::collections::HashMap<Vec<u8>, SequenceBuffer>>,
}

struct SequenceBuffer {
    next_id: i64,
    remaining: i64,
}

impl SequenceGenerator {
    pub fn open(config: &DatabaseConfig) -> Result<Self, DatabaseError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, &config.sequence_path)?;
        Ok(SequenceGenerator {
            db,
            cache: parking_lot::Mutex::new(std::collections::HashMap::new()),
        })
    }

    pub fn next(&self, queue_id: &[u8]) -> Result<i64, DatabaseError> {
        let mut cache = self.cache.lock();
        let buffer = cache.entry(queue_id.to_vec()).or_insert_with(|| SequenceBuffer {
            next_id: 0,
            remaining: 0,
        });
        if buffer.remaining <= 0 {
            let current = self.db.get(queue_id)?.map(|v| {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&v);
                i64::from_le_bytes(buf)
            }).unwrap_or(0);
            let next = current + 1000;
            self.db.put(queue_id, &next.to_le_bytes())?;
            buffer.next_id = current;
            buffer.remaining = 1000;
        }
        let id = buffer.next_id;
        buffer.next_id += 1;
        buffer.remaining -= 1;
        Ok(id)
    }
}