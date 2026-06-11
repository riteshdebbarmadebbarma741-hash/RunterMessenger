use rand::rngs::OsRng;
use rand::RngCore;
use crate::error::CryptoError;

pub fn generate_random_bytes(length: usize) -> Result<Vec<u8>, CryptoError> {
    let mut buffer = vec![0u8; length];
    OsRng.try_fill_bytes(&mut buffer).map_err(|_| CryptoError::RandomGenerationFailed)?;
    Ok(buffer)
}

pub fn generate_random_array<const N: usize>() -> Result<[u8; N], CryptoError> {
    let mut buffer = [0u8; N];
    OsRng.try_fill_bytes(&mut buffer).map_err(|_| CryptoError::RandomGenerationFailed)?;
    Ok(buffer)
}