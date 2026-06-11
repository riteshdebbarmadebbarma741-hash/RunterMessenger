// servers/rmp-server/src/error.rs  
use thiserror::Error;  
  
#[derive(Error, Debug)]  
pub enum RmpError {  
    #[error("Database error: {0}")]  
    Database(#[from] runter_database::DatabaseError),  
    #[error("Fencing error: {0}")]  
    Fencing(String),  
    #[error("Partition error: {0}")]  
    Partition(String),  
    #[error("Consumer error: {0}")]  
    Consumer(String),  
    #[error("Worker error: {0}")]  
    Worker(String),  
    #[error("Retry error: {0}")]  
    Retry(String),  
    #[error("DLQ error: {0}")]  
    Dlq(String),  
    #[error("Checkpoint error: {0}")]  
    Checkpoint(String),  
    #[error("Backpressure: {0}")]  
    Backpressure(String),  
    #[error("Stale epoch: expected={0}, got={1}")]  
    StaleEpoch(u64, u64),  
    #[error("Partition not owned")]  
    NotOwner,  
    #[error("Shutdown")]  
    Shutdown,  
}