// core/database/src/config.rs
#[derive(Clone)]
pub struct DatabaseConfig {
    pub wal_path: String,
    pub sqlite_path: String,
    pub rocks_path: String,
    pub sequence_path: String,
    pub batch_size: usize,
    pub max_queue_capacity: u64,
    pub backpressure_threshold: usize,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            wal_path: "runter_wal".into(),
            sqlite_path: "runter.db".into(),
            rocks_path: "runter_rocks".into(),
            sequence_path: "runter_seq".into(),
            batch_size: 500,
            max_queue_capacity: 100000,
            backpressure_threshold: 50000,
        }
    }
}