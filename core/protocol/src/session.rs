// core/protocol/src/session.rs
use crate::constants::{NONCE_TIMEOUT_SECS, MAX_QUEUES_PER_CONNECTION, MAX_NONCES_PER_SESSION, NONCE_CLEANUP_BATCH};
use crate::error::ProtocolError;
use crate::types::now_secs;
use std::collections::HashSet;

pub struct Session {
    pub identity_key: Vec<u8>,
    pub authenticated: bool,
    pub connected_at: u64,
    pub last_seen: u64,
    used_nonces: HashSet<Vec<u8>>,
    pub queue_count: usize,
    nonce_count_since_cleanup: usize,
    pub issued_nonces: Vec<Vec<u8>>,
}

impl Session {
    pub fn new(identity_key: Vec<u8>) -> Self {
        let now = now_secs();
        Session {
            identity_key,
            authenticated: false,
            connected_at: now,
            last_seen: now,
            used_nonces: HashSet::with_capacity(MAX_NONCES_PER_SESSION),
            queue_count: 0,
            nonce_count_since_cleanup: 0,
            issued_nonces: Vec::new(),
        }
    }

    pub fn issue_nonce(&mut self) -> Vec<u8> {
        let nonce = crate::types::generate_nonce();
        self.issued_nonces.push(nonce.clone());
        if self.issued_nonces.len() > MAX_NONCES_PER_SESSION {
            self.issued_nonces.remove(0);
        }
        nonce
    }

    pub fn verify_and_consume_nonce(&mut self, nonce: &[u8]) -> bool {
        if let Some(pos) = self.issued_nonces.iter().position(|n| n == nonce) {
            self.issued_nonces.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn check_and_store_nonce(&mut self, nonce: &[u8]) -> Result<(), ProtocolError> {
        let now = now_secs();
        if self.used_nonces.contains(nonce) {
            return Err(ProtocolError::NonceAlreadyUsed);
        }
        if nonce.len() < 12 {
            return Err(ProtocolError::InvalidFrame("Nonce too short".into()));
        }
        let ts = u64::from_be_bytes([
            nonce[0], nonce[1], nonce[2], nonce[3],
            nonce[4], nonce[5], nonce[6], nonce[7],
        ]);
        if now.saturating_sub(ts) > NONCE_TIMEOUT_SECS {
            return Err(ProtocolError::NonceExpired);
        }
        if self.used_nonces.len() >= MAX_NONCES_PER_SESSION {
            self.expire_old_nonces(now);
            if self.used_nonces.len() >= MAX_NONCES_PER_SESSION {
                return Err(ProtocolError::NonceAlreadyUsed);
            }
        }
        self.used_nonces.insert(nonce.to_vec());
        self.nonce_count_since_cleanup += 1;
        if self.nonce_count_since_cleanup >= NONCE_CLEANUP_BATCH {
            self.expire_old_nonces(now);
        }
        self.last_seen = now;
        Ok(())
    }

    fn expire_old_nonces(&mut self, now: u64) {
        self.used_nonces.retain(|n| {
            if n.len() < 8 { return false; }
            let ts = u64::from_be_bytes([n[0], n[1], n[2], n[3], n[4], n[5], n[6], n[7]]);
            now.saturating_sub(ts) < NONCE_TIMEOUT_SECS
        });
        self.nonce_count_since_cleanup = 0;
        self.issued_nonces.retain(|n| {
            if n.len() < 8 { return false; }
            let ts = u64::from_be_bytes([n[0], n[1], n[2], n[3], n[4], n[5], n[6], n[7]]);
            now.saturating_sub(ts) < NONCE_TIMEOUT_SECS
        });
    }

    pub fn check_queue_limit(&self) -> Result<(), ProtocolError> {
        if self.queue_count >= MAX_QUEUES_PER_CONNECTION {
            return Err(ProtocolError::TooManyQueues);
        }
        Ok(())
    }

    pub fn increment_queues(&mut self) { self.queue_count += 1; }
    pub fn decrement_queues(&mut self) { if self.queue_count > 0 { self.queue_count -= 1; } }
    pub fn is_expired(&self) -> bool { now_secs().saturating_sub(self.last_seen) > NONCE_TIMEOUT_SECS }
    pub fn touch(&mut self) { self.last_seen = now_secs(); }
}