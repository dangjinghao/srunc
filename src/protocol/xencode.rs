//! `XEncode` — the srun portal's XXTEA-variant block cipher, keyed by the
//! per-session challenge token.
//!
//! Note: srun's variant operates on a little-endian `u32` word array with the
//! plaintext byte-length appended as the final word, and serializes the result
//! back to little-endian bytes without truncation.

const DELTA: u32 = 0x9E37_79B9;

/// Encrypt `data` under `key` (the challenge token). Returns the raw cipher bytes.
/// An empty input yields an empty output, matching the reference implementation.
pub fn xencode(data: &[u8], key: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }
    let mut v = to_words(data, true);
    let mut k = to_words(key, false);
    // The round function indexes k[0..=3]; guard against degenerate short keys.
    while k.len() < 4 {
        k.push(0);
    }

    let n = v.len() - 1;
    let mut z = v[n];
    let mut sum: u32 = 0;
    let mut q: u32 = 6 + 52 / (n as u32 + 1);
    while q > 0 {
        sum = sum.wrapping_add(DELTA);
        let e = ((sum >> 2) & 3) as usize;
        for p in 0..=n {
            let y = if p == n { v[0] } else { v[p + 1] };
            let mut m = (z >> 5) ^ (y << 2);
            m = m.wrapping_add((y >> 3) ^ (z << 4) ^ (sum ^ y));
            m = m.wrapping_add(k[(p & 3) ^ e] ^ z);
            v[p] = v[p].wrapping_add(m);
            z = v[p];
        }
        q -= 1;
    }
    to_bytes(&v)
}

/// Pack bytes into little-endian `u32` words; optionally append the byte length.
fn to_words(a: &[u8], append_len: bool) -> Vec<u32> {
    let c = a.len();
    let mut v: Vec<u32> = vec![0; c.div_ceil(4)];
    let mut i = 0usize;
    while i < c {
        let mut t: u32 = 0;
        let mut j = 0usize;
        while j + i < c && j < 4 {
            t |= (a[i + j] as u32) << (j as u32 * 8);
            j += 1;
        }
        v[i >> 2] = t;
        i += 4;
    }
    if append_len {
        v.push(c as u32);
    }
    v
}

/// Serialize `u32` words back to little-endian bytes (no truncation).
fn to_bytes(a: &[u32]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(a.len() * 4);
    for &w in a {
        out.push((w & 0xff) as u8);
        out.push(((w >> 8) & 0xff) as u8);
        out.push(((w >> 16) & 0xff) as u8);
        out.push(((w >> 24) & 0xff) as u8);
    }
    out
}
