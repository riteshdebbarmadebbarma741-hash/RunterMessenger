// servers/rmp-server/src/fencing.rs
use crate::config::RmpConfig;
use crate::error::RmpError;
use std::sync::atomic::{AtomicU64, Ordering};
use std::fs;
use std::path::Path;

pub struct FencingToken {
    pub node_id: String,
    pub epoch: u64,
}

impl Clone for FencingToken {
    fn clone(&self) -> Self {
        FencingToken {
            node_id: self.node_id.clone(),
            epoch: self.epoch,
        }
    }
}

pub struct FenceManager {
    node_id: String,
    epoch: AtomicU64,
    epoch_file: String,
}

impl FenceManager {
    pub fn new(config: &RmpConfig) -> Result<Self, RmpError> {
        let epoch = if Path::new(&config.fencing_epoch_file).exists() {
            let data = fs::read_to_string(&config.fencing_epoch_file)?;
            data.trim().parse::<u64>().unwrap_or(0) + 1
        } else {
            1
        };
        fs::write(&config.fencing_epoch_file, epoch.to_string())?;
        Ok(FenceManager {
            node_id: config.node_id.clone(),
            epoch: AtomicU64::new(epoch),
            epoch_file: config.fencing_epoch_file.clone(),
        })
    }

    pub fn token(&self) -> FencingToken {
        FencingToken {
            node_id: self.node_id.clone(),
            epoch: self.epoch.load(Ordering::Acquire),
        }
    }

    pub fn increment_epoch(&self) -> Result<FencingToken, RmpError> {
        let new_epoch = self.epoch.fetch_add(1, Ordering::Release) + 1;
        fs::write(&self.epoch_file, new_epoch.to_string())?;
        Ok(FencingToken { node_id: self.node_id.clone(), epoch: new_epoch })
    }
}