use ed25519_dalek::{Signer, Verifier, Signature, SigningKey, VerifyingKey};
use crate::constants::ED25519_SIGNATURE_SIZE;
use crate::error::CryptoError;

pub fn sign(signing_key: &SigningKey, message: &[u8]) -> [u8; ED25519_SIGNATURE_SIZE] {
    signing_key.sign(message).to_bytes()
}

pub fn verify(verifying_key: &VerifyingKey, message: &[u8], signature: &[u8; ED25519_SIGNATURE_SIZE]) -> Result<(), CryptoError> {
    let sig = Signature::from_bytes(signature);
    verifying_key.verify(message, &sig).map_err(|_| CryptoError::SignatureVerificationFailed)
}