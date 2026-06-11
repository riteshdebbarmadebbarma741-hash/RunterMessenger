use zeroize::Zeroize;

pub fn secure_clear<T: Zeroize>(data: &mut T) {
    data.zeroize();
}

pub fn secure_clear_bytes(data: &mut [u8]) {
    data.zeroize();
}