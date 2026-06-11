use std::collections::HashMap;
use crate::aes_gcm_cipher;
use crate::hkdf_kdf;
use crate::constants::{AES_NONCE_SIZE, MAX_SKIP_MESSAGES, HEADER_LENGTH};
use crate::error::CryptoError;
use crate::keys::X25519KeyPair;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MessageKey {
    pub key: [u8; 32],
    pub nonce: [u8; AES_NONCE_SIZE],
    pub generation: u64,
}

pub struct RatchetState {
    pub root_key: [u8; 32],
    pub send_chain: ChainState,
    pub recv_chain: ChainState,
    pub our_dh: X25519KeyPair,
    pub their_dh_public: Option<[u8; 32]>,
    pub skipped_keys: HashMap<u64, MessageKey>,
}

pub struct ChainState {
    pub chain_key: [u8; 32],
    pub message_number: u64,
}

impl ChainState {
    pub fn advance(&mut self) -> [u8; 32] {
        let message_key = hkdf_kdf::derive(
            &self.chain_key,
            b"runter_message_key",
            &self.message_number.to_be_bytes(),
        ).expect("HKDF derive message key");
        let next_chain_key = hkdf_kdf::derive(
            &self.chain_key,
            b"runter_chain_key",
            &self.message_number.to_be_bytes(),
        ).expect("HKDF derive chain key");
        self.chain_key = next_chain_key;
        self.message_number = self.message_number.wrapping_add(1);
        message_key
    }
}

impl RatchetState {
    pub fn new(shared_secret: &[u8; 32], our_dh: X25519KeyPair) -> Result<Self, CryptoError> {
        let root_key = hkdf_kdf::derive(shared_secret, b"runter_root_key", &[])?;
        let send_chain_key = hkdf_kdf::derive(&root_key, b"runter_send_chain_init", &[])?;
        Ok(RatchetState {
            root_key,
            send_chain: ChainState { chain_key: send_chain_key, message_number: 0 },
            recv_chain: ChainState { chain_key: [0u8; 32], message_number: 0 },
            our_dh,
            their_dh_public: None,
            skipped_keys: HashMap::new(),
        })
    }

    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let message_key = self.send_chain.advance();
        let header = self.create_header();
        let mut result = vec![0u8; HEADER_LENGTH];
        result[..32].copy_from_slice(&self.our_dh.public_key);
        result[32..40].copy_from_slice(&self.send_chain.message_number.wrapping_sub(1).to_be_bytes());
        result[40..HEADER_LENGTH].copy_from_slice(&[0u8; 24]);

        let ciphertext = aes_gcm_cipher::encrypt(
            &message_key.key,
            &message_key.nonce,
            plaintext,
            &header,
        )?;

        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    pub fn decrypt(&mut self, message: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if message.len() < HEADER_LENGTH + 16 {
            return Err(CryptoError::DecryptionFailed);
        }
        let header = &message[..HEADER_LENGTH];
        let ciphertext = &message[HEADER_LENGTH..];
        let their_dh = header[..32].try_into().map_err(|_| CryptoError::HeaderDecryptionFailed)?;
        let message_number = u64::from_be_bytes(header[32..40].try_into().map_err(|_| CryptoError::HeaderDecryptionFailed)?);

        let needs_ratchet = match self.their_dh_public {
            Some(ref key) => key != their_dh,
            None => true,
        };

        if needs_ratchet {
            self.ratchet_forward(their_dh)?;
        }

        let gap = message_number.wrapping_sub(self.recv_chain.message_number);
        if gap > MAX_SKIP_MESSAGES {
            return Err(CryptoError::TooManySkippedMessages(gap));
        }

        while self.recv_chain.message_number < message_number {
            let skipped_key = self.recv_chain.advance();
            self.skipped_keys.insert(self.recv_chain.message_number.wrapping_sub(1), MessageKey {
                key: skipped_key,
                nonce: self.derive_nonce(self.recv_chain.message_number.wrapping_sub(1)),
                generation: self.recv_chain.message_number.wrapping_sub(1),
            });
        }

        let current_key = self.recv_chain.advance();
        let nonce = self.derive_nonce(message_number);

        aes_gcm_cipher::decrypt(&current_key, &nonce, ciphertext, header)
    }

    pub fn ratchet_forward(&mut self, their_dh: &[u8; 32]) -> Result<(), CryptoError> {
        let dh_secret = self.our_dh.diffie_hellman(their_dh);
        let new_root_key = hkdf_kdf::derive(
            &dh_secret,
            &self.root_key,
            b"runter_ratchet",
        )?;
        let new_send_chain_key = hkdf_kdf::derive(&new_root_key, b"runter_send_chain", &[])?;
        let new_dh = X25519KeyPair::generate()?;

        self.root_key = new_root_key;
        self.send_chain = ChainState { chain_key: new_send_chain_key, message_number: 0 };
        self.our_dh = new_dh;
        self.their_dh_public = Some(*their_dh);
        self.skipped_keys.clear();
        Ok(())
    }

    fn derive_nonce(&self, message_number: u64) -> [u8; AES_NONCE_SIZE] {
        let mut nonce = [0u8; AES_NONCE_SIZE];
        nonce[..8].copy_from_slice(&message_number.to_be_bytes());
        nonce
    }

    fn create_header(&self) -> Vec<u8> {
        let mut header = vec![0u8; HEADER_LENGTH];
        header[..32].copy_from_slice(&self.our_dh.public_key);
        header
    }
}