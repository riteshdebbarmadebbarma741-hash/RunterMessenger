// servers/rmp-server/src/checkpoint.rs
use crate::config::RmpConfig;
use crate::error::RmpError;
use runter_database::materializer::Materializer;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};

pub struct CheckpointManager {
    materializer: Arc<Materializer>,
}

pub struct Checkpoint {
    pub index: u64,
    pub hash: String,
    pub timestamp: u64,
}

impl CheckpointManager {
    pub fn new(materializer: Arc<Materializer>) -> Self {
        CheckpointManager { materializer }
    }

    pub fn create(&self) -> Result<Checkpoint, RmpError> {
        let index = self.materializer.get_last_applied_index()?;
        let mut hasher = Sha256::new();
        hasher.update(&index.to_be_bytes());
        hasher.update(&SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos().to_be_bytes());
        let hash = hex::encode(hasher.finalize());
        Ok(Checkpoint {
            index,
            hash,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        })
    }
}