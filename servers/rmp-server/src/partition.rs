// servers/rmp-server/src/partition.rs
use crate::config::RmpConfig;
use crate::error::RmpError;
use crate::fencing::FencingToken;
use dashmap::DashMap;
use std::sync::Arc;

pub struct Partition {
    pub id: usize,
    pub start_index: u64,
    pub end_index: u64,
    pub owner_epoch: u64,
    pub owner_node: String,
}

pub struct PartitionManager {
    partitions: DashMap<usize, Partition>,
    config: RmpConfig,
}

impl PartitionManager {
    pub fn new(config: &RmpConfig) -> Self {
        let partitions = DashMap::new();
        for i in 0..config.partition_count {
            partitions.insert(i, Partition {
                id: i,
                start_index: 0,
                end_index: 0,
                owner_epoch: 0,
                owner_node: String::new(),
            });
        }
        PartitionManager {
            partitions,
            config: config.clone(),
        }
    }

    pub fn claim(&self, partition_id: usize, token: &FencingToken, start: u64, end: u64) -> Result<(), RmpError> {
        let mut partition = self.partitions.get_mut(&partition_id)
            .ok_or(RmpError::Partition("Partition not found".into()))?;
        if partition.owner_epoch >= token.epoch {
            return Err(RmpError::StaleEpoch(partition.owner_epoch, token.epoch));
        }
        partition.owner_epoch = token.epoch;
        partition.owner_node = token.node_id.clone();
        partition.start_index = start;
        partition.end_index = end;
        Ok(())
    }

    pub fn release(&self, partition_id: usize, token: &FencingToken) -> Result<(), RmpError> {
        let mut partition = self.partitions.get_mut(&partition_id)
            .ok_or(RmpError::Partition("Partition not found".into()))?;
        if partition.owner_epoch != token.epoch {
            return Err(RmpError::StaleEpoch(partition.owner_epoch, token.epoch));
        }
        partition.owner_epoch = 0;
        partition.owner_node.clear();
        Ok(())
    }

    pub fn is_owner(&self, partition_id: usize, token: &FencingToken) -> bool {
        self.partitions.get(&partition_id)
            .map(|p| p.owner_epoch == token.epoch && p.owner_node == token.node_id)
            .unwrap_or(false)
    }

    pub fn assign_work(&self, last_applied: u64) -> Vec<(usize, u64, u64)> {
        let total_work = self.config.consumer_batch_size as u64;
        let work_per_partition = total_work / self.config.partition_count as u64;
        let mut assignments = Vec::new();
        for i in 0..self.config.partition_count {
            let start = last_applied + (i as u64 * work_per_partition) + 1;
            let end = start + work_per_partition - 1;
            assignments.push((i, start, end));
        }
        assignments
    }
}