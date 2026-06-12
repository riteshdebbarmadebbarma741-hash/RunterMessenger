// core/agent/src/config.rs
#[derive(Clone)]
pub struct AgentConfig {
    pub display_name: String,
    pub db_passphrase: String,
    pub relay_servers: Vec<String>,
    pub enable_tor: bool,
    pub tor_proxy: Option<String>,
    pub connection_timeout_secs: u64,
    pub retry_max_attempts: u32,
    pub retry_base_delay_ms: u64,
    pub max_message_size: usize,
    pub session_store_path: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        AgentConfig {
            display_name: String::new(),
            db_passphrase: String::new(),
            relay_servers: vec!["rmp.runter.chat:443".into()],
            enable_tor: false,
            tor_proxy: None,
            connection_timeout_secs: 30,
            retry_max_attempts: 5,
            retry_base_delay_ms: 100,
            max_message_size: 65536,
            session_store_path: "runter_sessions.db".into(),
        }
    }
}