use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Key generation failed")]
    KeyGenerationFailed,
    #[error("Invalid key length")]
    InvalidKeyLength,
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    #[error("Random generation failed")]
    RandomGenerationFailed,
    #[error("HKDF derivation failed")]
    HkdfDerivationFailed,
    #[error("HMAC computation failed")]
    HmacComputationFailed,
    #[error("Too many skipped messages: {0}")]
    TooManySkippedMessages(u64),
    #[error("X3DH key exchange failed")]
    X3DHFailed,
    #[error("Identity key mismatch")]
    IdentityKeyMismatch,
    #[error("Header decryption failed")]
    HeaderDecryptionFailed,
}