use hmac::{Hmac, Mac};
use sha2::Sha256;
use crate::error::CryptoError;

type HmacSha256 = Hmac<Sha256>;

pub fn compute(key: &[u8], message: &[u8]) -> Result<[u8; 32], CryptoError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| CryptoError::HmacComputationFailed)?;
    mac.update(message);
    let result = mac.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(&result.into_bytes());
    Ok(output)
}