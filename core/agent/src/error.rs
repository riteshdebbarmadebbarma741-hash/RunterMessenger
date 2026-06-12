// core/agent/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Crypto error: {0}")]
    Crypto(#[from] runter_crypto::CryptoError),
    #[error("Protocol error: {0}")]
    Protocol(#[from] runter_protocol::ProtocolError),
    #[error("Database error: {0}")]
    Database(#[from] runter_database::DatabaseError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Session error: {0}")]
    Session(String),
    #[error("Handshake error: {0}")]
    Handshake(String),
    #[error("Identity error: {0}")]
    Identity(String),
    #[error("Queue error: {0}")]
    Queue(String),
    #[error("Message error: {0}")]
    Message(String),
    #[error("Relay error: {0}")]
    Relay(String),
    #[error("Retry exhausted: {0}")]
    RetryExhausted(String),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Not connected")]
    NotConnected,
    #[error("Timeout")]
    Timeout,
    #[error("Invalid key length: expected {0}, got {1}")]
    InvalidKeyLength(usize, usize),
}