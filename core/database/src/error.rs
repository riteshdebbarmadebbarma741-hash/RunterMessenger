// core/database/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("WAL error: {0}")]
    Wal(String),
    #[error("Sequence error: {0}")]
    Sequence(String),
    #[error("Materializer error: {0}")]
    Materializer(String),
    #[error("Queue full: {0}")]
    QueueFull(String),
    #[error("Backpressure: {0}")]
    Backpressure(String),
    #[error("CRC mismatch at index {index}")]
    CrcMismatch { index: u64 },
    #[error("Entry not found: {0}")]
    NotFound(String),
}