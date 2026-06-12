// core/agent/src/lib.rs
pub mod config;
pub mod error;
pub mod types;
pub mod identity;
pub mod state;
pub mod session;
pub mod session_store;
pub mod handshake;
pub mod encryption;
pub mod connection;
pub mod message;
pub mod queue_pair;
pub mod presence;
pub mod invite;
pub mod relay;
pub mod transport;
pub mod retry;
pub mod notification;
pub mod backup;
pub mod discovery;
pub mod group;
pub mod channel;
pub mod bot;
pub mod search;
pub mod settings;
pub mod incognito;
pub mod metrics;
pub mod logging;

pub use config::AgentConfig;
pub use error::AgentError;
pub use identity::AgentIdentity;
pub use session::AgentSession;
pub use handshake::{
    create_invitation,
    build_handshake_response,
    verify_handshake_response,
    complete_handshake_as_initiator,
    complete_handshake_as_responder,
};
pub use encryption::EncryptionManager;
pub use connection::ConnectionManager;
pub use message::MessageHandler;