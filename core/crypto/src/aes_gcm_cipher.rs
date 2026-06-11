use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce, Key,
};
use crate::constants::AES_NONCE_SIZE;
use crate::error::CryptoError;

pub fn encrypt(key: &[u8; 32], nonce: &[u8; AES_NONCE_SIZE], plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher_key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(cipher_key);
    let nonce = Nonce::from_slice(nonce);
    let payload = Payload { msg: plaintext, aad };
    cipher.encrypt(nonce, payload).map_err(|_| CryptoError::EncryptionFailed)
}

pub fn decrypt(key: &[u8; 32], nonce: &[u8; AES_NONCE_SIZE], ciphertext: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher_key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(cipher_key);
    let nonce = Nonce::from_slice(nonce);
    let payload = Payload { msg: ciphertext, aad };
    cipher.decrypt(nonce, payload).map_err(|_| CryptoError::DecryptionFailed)
}