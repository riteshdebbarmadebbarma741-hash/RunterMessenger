// core/database/src/lib.rs
pub mod config;
pub mod error;
pub mod wal;
pub mod sequence;
pub mod materializer;
pub mod queue_store;
pub mod message_store;
pub mod connection_store;
pub mod user_store;
pub mod metrics;
pub mod backpressure;

pub use error::DatabaseError;
pub use config::DatabaseConfig;
pub use wal::WriteAheadLog;
pub use sequence::SequenceGenerator;
pub use materializer::Materializer;
pub use message_store::MessageStore;
pub use queue_store::QueueStore;
pub use metrics::DatabaseMetrics;

use std::sync::Arc;

pub struct Database {
    pub wal: Arc<WriteAheadLog>,
    pub materializer: Arc<Materializer>,
    pub sequence: Arc<SequenceGenerator>,
    pub config: DatabaseConfig,
    pub metrics: Arc<DatabaseMetrics>,
}

impl Database {
    pub fn new(config: DatabaseConfig) -> Result<Self, DatabaseError> {
        let metrics = Arc::new(DatabaseMetrics::register()?);
        let wal = Arc::new(WriteAheadLog::open(&config, &metrics)?);
        let sequence = Arc::new(SequenceGenerator::open(&config)?);
        let materializer = Arc::new(Materializer::open(&config, &metrics)?);

        let db = Database { wal, materializer, sequence, config, metrics };
        db.recover()?;
        Ok(db)
    }

    fn recover(&self) -> Result<(), DatabaseError> {
        let _timer = self.metrics.recovery_duration_seconds.start_timer();
        let last_applied = self.materializer.get_last_applied_index()?;
        let entries = self.wal.read_from(last_applied + 1)?;
        self.metrics.recovery_entries.set(entries.len() as i64);
        for entry in entries {
            self.materializer.apply(&entry)?;
            self.wal.mark_applied(&entry.index)?;
        }
        Ok(())
    }
}