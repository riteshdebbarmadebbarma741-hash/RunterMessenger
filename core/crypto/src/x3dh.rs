use crate::hkdf_kdf;
use crate::keys::{IdentityKeyPair, X25519KeyPair, PreKey, SignedPreKey};
use crate::error::CryptoError;

pub struct X3DHResult {
    pub shared_secret: [u8; 32],
    pub associated_data: Vec<u8>,
    pub ephemeral_public_key: [u8; 32],
}

pub fn x3dh_send(
    our_identity: &IdentityKeyPair,
    our_ephemeral: &X25519KeyPair,
    their_identity_public: &[u8; 32],
    their_signed_prekey_public: &[u8; 32],
    their_one_time_prekey_public: Option<&[u8; 32]>,
) -> Result<X3DHResult, CryptoError> {
    let our_x25519 = our_identity.derive_x25519_keypair();
    let dh1 = our_x25519.diffie_hellman(their_signed_prekey_public);
    let dh2 = our_ephemeral.diffie_hellman(their_identity_public);
    let dh3 = our_ephemeral.diffie_hellman(their_signed_prekey_public);

    let mut secret_input = Vec::with_capacity(128);
    secret_input.extend_from_slice(&dh1);
    secret_input.extend_from_slice(&dh2);
    secret_input.extend_from_slice(&dh3);

    if let Some(prekey) = their_one_time_prekey_public {
        let dh4 = our_ephemeral.diffie_hellman(prekey);
        secret_input.extend_from_slice(&dh4);
    }

    let shared_secret = hkdf_kdf::derive(&secret_input, b"runter_x3dh_shared_secret", &[])?;

    let mut associated_data = Vec::new();
    associated_data.extend_from_slice(our_identity.verifying_key.as_bytes());
    associated_data.extend_from_slice(their_identity_public);

    Ok(X3DHResult {
        shared_secret,
        associated_data,
        ephemeral_public_key: our_ephemeral.public_key,
    })
}

pub fn x3dh_receive(
    our_identity: &IdentityKeyPair,
    our_signed_prekey: &SignedPreKey,
    their_identity_public: &[u8; 32],
    their_ephemeral_public: &[u8; 32],
    our_one_time_prekey: Option<&PreKey>,
) -> Result<X3DHResult, CryptoError> {
    let our_x25519 = our_identity.derive_x25519_keypair();
    let dh1 = our_signed_prekey.key_pair.diffie_hellman(their_identity_public);
    let dh2 = our_x25519.diffie_hellman(their_ephemeral_public);
    let dh3 = our_signed_prekey.key_pair.diffie_hellman(their_ephemeral_public);

    let mut secret_input = Vec::with_capacity(128);
    secret_input.extend_from_slice(&dh1);
    secret_input.extend_from_slice(&dh2);
    secret_input.extend_from_slice(&dh3);

    if let Some(prekey) = our_one_time_prekey {
        let dh4 = prekey.key_pair.diffie_hellman(their_ephemeral_public);
        secret_input.extend_from_slice(&dh4);
    }

    let shared_secret = hkdf_kdf::derive(&secret_input, b"runter_x3dh_shared_secret", &[])?;

    let mut associated_data = Vec::new();
    associated_data.extend_from_slice(their_identity_public);
    associated_data.extend_from_slice(our_identity.verifying_key.as_bytes());

    Ok(X3DHResult {
        shared_secret,
        associated_data,
        ephemeral_public_key: *their_ephemeral_public,
    })
}