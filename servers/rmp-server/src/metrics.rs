// servers/rmp-server/src/metrics.rs
use crate::error::RmpError;
use prometheus::{Counter, Registry};
use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    pub static ref RMP_REGISTRY: Registry = Registry::default();
}

#[derive(Clone)]
pub struct RmpMetrics {
    pub consumer_batches: Counter,
    pub consumer_entries: Counter,
    pub worker_processed: Counter,
    pub worker_errors: Counter,
    pub retry_scheduled: Counter,
    pub retry_exhausted: Counter,
    pub dlq_pushed: Counter,
    pub checkpoint_created: Counter,
    pub backpressure_events: Counter,
}

impl RmpMetrics {
    pub fn register() -> Result<Self, RmpError> {
        let consumer_batches = Counter::new("rmp_consumer_batches_total", "Consumer batches")?;
        let consumer_entries = Counter::new("rmp_consumer_entries_total", "Consumer entries")?;
        let worker_processed = Counter::new("rmp_worker_processed_total", "Worker processed")?;
        let worker_errors = Counter::new("rmp_worker_errors_total", "Worker errors")?;
        let retry_scheduled = Counter::new("rmp_retry_scheduled_total", "Retries scheduled")?;
        let retry_exhausted = Counter::new("rmp_retry_exhausted_total", "Retries exhausted")?;
        let dlq_pushed = Counter::new("rmp_dlq_pushed_total", "DLQ pushed")?;
        let checkpoint_created = Counter::new("rmp_checkpoint_created_total", "Checkpoints")?;
        let backpressure_events = Counter::new("rmp_backpressure_events_total", "Backpressure events")?;

        RMP_REGISTRY.register(Box::new(consumer_batches.clone()))?;
        RMP_REGISTRY.register(Box::new(consumer_entries.clone()))?;
        RMP_REGISTRY.register(Box::new(worker_processed.clone()))?;
        RMP_REGISTRY.register(Box::new(worker_errors.clone()))?;
        RMP_REGISTRY.register(Box::new(retry_scheduled.clone()))?;
        RMP_REGISTRY.register(Box::new(retry_exhausted.clone()))?;
        RMP_REGISTRY.register(Box::new(dlq_pushed.clone()))?;
        RMP_REGISTRY.register(Box::new(checkpoint_created.clone()))?;
        RMP_REGISTRY.register(Box::new(backpressure_events.clone()))?;

        Ok(RmpMetrics {
            consumer_batches, consumer_entries, worker_processed, worker_errors,
            retry_scheduled, retry_exhausted, dlq_pushed, checkpoint_created, backpressure_events,
        })
    }
}