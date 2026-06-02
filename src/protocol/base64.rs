//! `QuirkBase64Encode` — base64 with srun's shuffled alphabet and a couple of
//! quirks (it reads the 3-byte group in reverse and pads slightly differently).
//! Faithful port of `libauth.QuirkBase64Encode` (coding.go).

const ALPHABET: &[u8; 64] = b"LVoJPiCN2R8G90yg+hmFHuacZ1OWMnrsSTXkYpUq/3dlbfKwv6xztjI7DeBE45QA";

/// Encode arbitrary bytes using srun's custom base64 variant.
pub fn quirk_base64(t: &[u8]) -> String {
    let a = t.len();
    let mut out: Vec<u8> = Vec::new();
    let mut o = 0usize;
    while o < a {
        let p2 = t[o] as u32;
        let p1 = if o + 1 < a { t[o + 1] as u32 } else { 0 };
        let p0 = if o + 2 < a { t[o + 2] as u32 } else { 0 };
        let h = (p2 << 16) | (p1 << 8) | p0;
        for i in 0..4usize {
            if o * 8 + i * 6 > a * 8 {
                out.push(b'=');
            } else {
                let idx = ((h >> (6 * (3 - i) as u32)) & 0x3f) as usize;
                out.push(ALPHABET[idx]);
            }
        }
        o += 3;
    }
    // Every byte pushed is from ALPHABET or '=', so this is always valid UTF-8.
    String::from_utf8(out).unwrap_or_default()
}
