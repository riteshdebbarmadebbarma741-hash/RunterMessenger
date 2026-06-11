// core/protocol/src/constants.rs
pub const PROTOCOL_VERSION: u8 = 1;
pub const MAX_FRAME_SIZE: usize = 65536;
pub const MAX_PAYLOAD_SIZE: usize = 65536;
pub const MAX_QUEUE_NAME_LENGTH: usize = 64;
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
pub const KEEPALIVE_INTERVAL_SECS: u64 = 15;
pub const MAX_BATCH_SIZE: usize = 100;
pub const MAX_QUEUES_PER_CONNECTION: usize = 1000;
pub const NONCE_LENGTH: usize = 16;
pub const NONCE_TIMEOUT_SECS: u64 = 300;
pub const CLEANUP_INTERVAL_SECS: u64 = 60;
pub const MAX_MESSAGES_PER_QUEUE: u64 = 10000;
pub const BACKPRESSURE_THRESHOLD: usize = 500;
pub const MAX_SUBSCRIBERS_PER_QUEUE: usize = 100;
pub const MAX_NONCES_PER_SESSION: usize = 10000;
pub const NONCE_CLEANUP_BATCH: usize = 1000;
pub const TRANSPORT_TIMEOUT_SECS: u64 = 30;