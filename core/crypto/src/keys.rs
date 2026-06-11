use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use sha2::{Sha512, Digest};
use zeroize::{Zeroize, ZeroizeOnDrop};
use crate::error::CryptoError;
use crate::constants::ED25519_SIGNATURE_SIZE;

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct X25519KeyPair {
    pub private_key: [u8; 32],
    pub public_key: [u8; 32],
}

impl X25519KeyPair {
    pub fn generate() -> Result<Self, CryptoError> {
        let secret = x25519_dalek::StaticSecret::random_from_rng(OsRng);
        let public = x25519_dalek::PublicKey::from(&secret);
        Ok(X25519KeyPair {
            private_key: secret.to_bytes(),
            public_key: public.to_bytes(),
        })
    }

    pub fn from_private_key(private_key: &[u8; 32]) -> Self {
        let secret = x25519_dalek::StaticSecret::from(*private_key);
        let public = x25519_dalek::PublicKey::from(&secret);
        X25519KeyPair {
            private_key: *private_key,
            public_key: public.to_bytes(),
        }
    }

    pub fn diffie_hellman(&self, their_public: &[u8; 32]) -> [u8; 32] {
        let secret = x25519_dalek::StaticSecret::from(self.private_key);
        let public = x25519_dalek::PublicKey::from(*their_public);
        *secret.diffie_hellman(&public).as_bytes()
    }
}

pub struct IdentityKeyPair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

impl IdentityKeyPair {
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        IdentityKeyPair { signing_key, verifying_key }
    }

    pub fn sign(&self, message: &[u8]) -> [u8; ED25519_SIGNATURE_SIZE] {
        self.signing_key.sign(message).to_bytes()
    }

    pub fn verify(&self, message: &[u8], signature: &[u8; ED25519_SIGNATURE_SIZE]) -> Result<(), CryptoError> {
        let sig = Signature::from_bytes(signature);
        self.verifying_key.verify(message, &sig).map_err(|_| CryptoError::SignatureVerificationFailed)
    }

    pub fn to_x25519_public(&self) -> [u8; 32] {
        let ed_point = curve25519_dalek::edwards::CompressedEdwardsY::from_slice(self.verifying_key.as_bytes())
            .decompress()
            .expect("Valid Ed25519 point");
        ed_point.to_montgomery().to_bytes()
    }

    pub fn to_x25519_private(&self) -> [u8; 32] {
        let mut hasher = Sha512::new();
        hasher.update(self.signing_key.as_bytes());
        let hash = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash[..32]);
        key[0] &= 248;
        key[31] &= 127;
        key[31] |= 64;
        key
    }

    pub fn derive_x25519_keypair(&self) -> X25519KeyPair {
        let private = self.to_x25519_private();
        X25519KeyPair::from_private_key(&private)
    }
}

pub struct PreKey {
    pub id: u32,
    pub key_pair: X25519KeyPair,
}

impl PreKey {
    pub fn generate(id: u32) -> Result<Self, CryptoError> {
        Ok(PreKey { id, key_pair: X25519KeyPair::generate()? })
    }
}

pub struct SignedPreKey {
    pub id: u32,
    pub key_pair: X25519KeyPair,
    pub signature: [u8; ED25519_SIGNATURE_SIZE],
}

impl SignedPreKey {
    pub fn generate(id: u32, identity: &IdentityKeyPair) -> Result<Self, CryptoError> {
        let key_pair = X25519KeyPair::generate()?;
        let signature = identity.sign(&key_pair.public_key);
        Ok(SignedPreKey { id, key_pair, signature })
    }

    pub fn verify(&self, identity: &IdentityKeyPair) -> Result<(), CryptoError> {
        identity.verify(&self.key_pair.public_key, &self.signature)
    }
}