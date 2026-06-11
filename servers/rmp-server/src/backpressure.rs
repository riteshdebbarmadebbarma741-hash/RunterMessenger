// servers/rmp-server/src/backpressure.rs
use crate::config::RmpConfig;
use crate::error::RmpError;
use runter_database::materializer::Materializer;
use runter_database::wal::WriteAheadLog;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct BackpressureController {
    lag_threshold: u64,
    current_lag: AtomicU64,
    wal: Arc<WriteAheadLog>,
    materializer: Arc<Materializer>,
}

impl BackpressureController {
    pub fn new(config: &RmpConfig, wal: Arc<WriteAheadLog>, materializer: Arc<Materializer>) -> Self {
        BackpressureController {
            lag_threshold: config.backpressure_lag_threshold,
            current_lag: AtomicU64::new(0),
            wal,
            materializer,
        }
    }

    pub fn check(&self) -> Result<(), RmpError> {
        let applied = self.materializer.get_last_applied_index().unwrap_or(0);
        let lag = applied.saturating_sub(applied);
        self.current_lag.store(lag, Ordering::Release);
        if lag > self.lag_threshold {
            return Err(RmpError::Backpressure(format!("Lag {} > threshold {}", lag, self.lag_threshold)));
        }
        Ok(())
    }
}