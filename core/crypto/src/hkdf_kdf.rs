use hkdf::Hkdf;
use sha2::Sha256;
use crate::error::CryptoError;

pub fn derive(ikm: &[u8], salt: &[u8], info: &[u8]) -> Result<[u8; 32], CryptoError> {
    let hkdf = Hkdf::<Sha256>::new(Some(salt), ikm);
    let mut output = [0u8; 32];
    hkdf.expand(info, &mut output).map_err(|_| CryptoError::HkdfDerivationFailed)?;
    Ok(output)
}

pub fn derive_to_slice(ikm: &[u8], salt: &[u8], info: &[u8], output: &mut [u8]) -> Result<(), CryptoError> {
    let hkdf = Hkdf::<Sha256>::new(Some(salt), ikm);
    hkdf.expand(info, output).map_err(|_| CryptoError::HkdfDerivationFailed)?;
    Ok(())
}