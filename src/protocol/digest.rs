//! MD5 / SHA1 lowercase-hex helpers, backed by the audited RustCrypto
//! implementations.

use md5::Md5;
use sha1::{Digest, Sha1};

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::new();
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

/// Lowercase hex of SHA1(data).
pub fn sha1_hex(data: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(data);
    to_hex(h.finalize().as_slice())
}

/// Lowercase hex of MD5(data).
pub fn md5_hex(data: &[u8]) -> String {
    let mut h = Md5::new();
    h.update(data);
    to_hex(h.finalize().as_slice())
}
