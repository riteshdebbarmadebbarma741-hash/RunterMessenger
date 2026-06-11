// servers/rmp-server/src/server.rs
use crate::config::RmpConfig;
use crate::error::RmpError;
use crate::fencing::FenceManager;
use crate::partition::PartitionRouter;
use crate::offset_index::OffsetIndex;
use crate::consumer::PartitionConsumer;
use crate::worker::{Worker, ApplyResult};
use crate::retry::RetryManager;
use crate::dlq::DeadLetterQueue;
use crate::idempotency::IdempotencyRegistry;
use crate::backpressure::BackpressureController;
use crate::checkpoint::CheckpointManager;
use crate::metrics::RmpMetrics;
use runter_database::Database;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct RmpServer {
    config: RmpConfig,
    metrics: Arc<RmpMetrics>,
}

impl RmpServer {
    pub fn new(config: RmpConfig, db: Arc<Database>) -> Result<Self, RmpError> {
        let metrics = Arc::new(RmpMetrics::register()?);
        let fence_manager = Arc::new(FenceManager::new(&config)?);
        let router = Arc::new(PartitionRouter::new(config.partition_count));
        let offset_index = Arc::new(OffsetIndex::open("rmp_offset_index")?);
        let idempotency = Arc::new(IdempotencyRegistry::open("rmp_idempotency")?);
        let retry_manager = Arc::new(RetryManager::new(db.wal.clone(), &config));
        let dlq = Arc::new(DeadLetterQueue::new(db.wal.clone()));
        let backpressure = Arc::new(BackpressureController::new(&config, db.wal.clone(), db.materializer.clone()));
        let checkpoint_manager = Arc::new(CheckpointManager::new(db.materializer.clone()));

        let token = fence_manager.token();

        for partition_id in 0..config.partition_count {
            let (dispatch_tx, dispatch_rx) = mpsc::unbounded_channel();
            let (result_tx, mut result_rx) = mpsc::unbounded_channel();

            let wal = db.wal.clone();
            tokio::spawn(async move {
                while let Some(result) = result_rx.recv().await {
                    if result.success {
                        let _ = wal.mark_applied(&result.index);
                    }
                }
            });

            let consumer = PartitionConsumer {
                partition_id,
                token: token.clone(),
                wal: db.wal.clone(),
                offset_index: offset_index.clone(),
                materializer: db.materializer.clone(),
                router: router.clone(),
                dispatch_tx: dispatch_tx.clone(),
                config: config.clone(),
                metrics: metrics.clone(),
            };
            tokio::spawn(async move { consumer.run().await });

            let worker = Worker {
                id: partition_id,
                rx: dispatch_rx,
                result_tx,
                materializer: db.materializer.clone(),
                idempotency: idempotency.clone(),
                retry_manager: retry_manager.clone(),
                dlq: dlq.clone(),
                db: db.clone(),
                config: config.clone(),
                metrics: metrics.clone(),
            };
            tokio::spawn(async move { worker.run().await });
        }

        let checkpoint_clone = checkpoint_manager;
        let metrics_clone = metrics.clone();
        let interval = config.checkpoint_interval_secs;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
                if checkpoint_clone.create().is_ok() {
                    metrics_clone.checkpoint_created.inc();
                }
            }
        });

        Ok(RmpServer { config, metrics })
    }
}