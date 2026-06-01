//! MD5 / SHA1 lowercase-hex helpers. We reuse the audited RustCrypto
//! implementations rather than hand-rolling hashes (matching Go's
//! `crypto/md5` + `crypto/sha1`).

use alloc::string::String;
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

/// Lowercase hex of SHA1(data). Mirrors `libauth.sha1sum`.
pub fn sha1_hex(data: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(data);
    to_hex(h.finalize().as_slice())
}

/// Lowercase hex of MD5(data). Mirrors Go's `fmt.Sprintf("%032x", md5.Sum(..))`.
pub fn md5_hex(data: &[u8]) -> String {
    let mut h = Md5::new();
    h.update(data);
    to_hex(h.finalize().as_slice())
}
