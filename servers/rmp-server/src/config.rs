// servers/rmp-server/src/config.rs
#[derive(Clone)]
pub struct RmpConfig {
    pub node_id: String,
    pub partition_count: usize,
    pub consumer_batch_size: usize,
    pub consumer_poll_interval_ms: u64,
    pub worker_threads: usize,
    pub retry_max_attempts: u32,
    pub retry_base_delay_ms: u64,
    pub retry_max_delay_ms: u64,
    pub fencing_epoch_file: String,
    pub checkpoint_interval_secs: u64,
    pub backpressure_lag_threshold: u64,
}

impl Default for RmpConfig {
    fn default() -> Self {
        RmpConfig {
            node_id: format!("node-{}", uuid::Uuid::new_v4()),
            partition_count: 16,
            consumer_batch_size: 500,
            consumer_poll_interval_ms: 100,
            worker_threads: 8,
            retry_max_attempts: 5,
            retry_base_delay_ms: 100,
            retry_max_delay_ms: 30000,
            fencing_epoch_file: "rmp_epoch".into(),
            checkpoint_interval_secs: 60,
            backpressure_lag_threshold: 10000,
        }
    }
}