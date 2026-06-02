//! Assembly of the srun portal login/logout parameters. Faithful port of
//! `libauth.buildLoginParams` (coding.go), kept as a pure function so it can be
//! pinned to the Go reference output in tests.

use super::base64::quirk_base64;
use super::digest::{md5_hex, sha1_hex};
use super::xencode::xencode;

const ENC_VER: &str = "srun_bx1";
const N: &str = "200";
const TYPE: &str = "1";
const DOUBLE_STACK: &str = "1";

/// The account being authenticated.
pub struct Credentials<'a> {
    pub username: &'a str,
    pub password: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Login,
    Logout,
}

/// The parameters to send to `/cgi-bin/srun_portal`. Field names mirror the
/// query keys the portal expects.
#[derive(Clone, Debug)]
pub struct PortalParams {
    pub action: String,
    pub username: String,
    /// `{MD5}<hex>` for login, `None` for logout.
    pub password: Option<String>,
    pub ac_id: String,
    pub ip: String,
    pub info: String,
    pub chksum: String,
    pub n: String,
    pub type_: String,
    pub double_stack: String,
}

impl PortalParams {
    /// The (key, value) pairs to URL-encode, in a stable order. The imperative
    /// shell turns these into the actual query string.
    pub fn to_pairs(&self) -> Vec<(&str, &str)> {
        let mut pairs = vec![
            ("action", self.action.as_str()),
            ("username", self.username.as_str()),
            ("ac_id", self.ac_id.as_str()),
            ("ip", self.ip.as_str()),
            ("info", self.info.as_str()),
            ("chksum", self.chksum.as_str()),
            ("n", self.n.as_str()),
            ("type", self.type_.as_str()),
            ("double_stack", self.double_stack.as_str()),
        ];
        if let Some(pw) = &self.password {
            pairs.push(("password", pw.as_str()));
        }
        pairs
    }
}

/// Build the portal parameters for a login or logout, given the challenge `token`.
pub fn build_portal_params(
    creds: &Credentials,
    token: &str,
    action: Action,
    ip: &str,
    ac_id: &str,
) -> PortalParams {
    let logout = action == Action::Logout;
    let hmd5 = md5_hex(creds.password.as_bytes());

    let info_json = build_info_json(creds, ip, ac_id, logout);
    let mut info = String::from("{SRBX1}");
    info.push_str(&quirk_base64(&xencode(
        info_json.as_bytes(),
        token.as_bytes(),
    )));

    let chksum = if logout {
        sha1_hex(
            join(&[
                token,
                creds.username,
                token,
                ac_id,
                token,
                ip,
                token,
                N,
                token,
                TYPE,
                token,
                &info,
            ])
            .as_bytes(),
        )
    } else {
        sha1_hex(
            join(&[
                token,
                creds.username,
                token,
                &hmd5,
                token,
                ac_id,
                token,
                ip,
                token,
                N,
                token,
                TYPE,
                token,
                &info,
            ])
            .as_bytes(),
        )
    };

    let password = if logout {
        None
    } else {
        let mut p = String::from("{MD5}");
        p.push_str(&hmd5);
        Some(p)
    };

    PortalParams {
        action: String::from(if logout { "logout" } else { "login" }),
        username: String::from(creds.username),
        password,
        ac_id: String::from(ac_id),
        ip: String::from(ip),
        info,
        chksum,
        n: String::from(N),
        type_: String::from(TYPE),
        double_stack: String::from(DOUBLE_STACK),
    }
}

fn join(parts: &[&str]) -> String {
    let mut s = String::new();
    for p in parts {
        s.push_str(p);
    }
    s
}

/// Reproduce Go's `json.Marshal(map[string]string{...})`: keys are emitted in
/// sorted order (acid, enc_ver, ip, [password,] username) and `< > &` are
/// `\uXXXX`-escaped, so the encrypted blob is byte-identical to the reference.
fn build_info_json(creds: &Credentials, ip: &str, ac_id: &str, logout: bool) -> String {
    let mut s = String::from("{");
    push_kv(&mut s, "acid", ac_id);
    s.push(',');
    push_kv(&mut s, "enc_ver", ENC_VER);
    s.push(',');
    push_kv(&mut s, "ip", ip);
    if !logout {
        s.push(',');
        push_kv(&mut s, "password", creds.password);
    }
    s.push(',');
    push_kv(&mut s, "username", creds.username);
    s.push('}');
    s
}

fn push_kv(s: &mut String, k: &str, v: &str) {
    push_json_str(s, k);
    s.push(':');
    push_json_str(s, v);
}

fn push_json_str(s: &mut String, v: &str) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    s.push('"');
    for ch in v.chars() {
        match ch {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            '<' => s.push_str("\\u003c"),
            '>' => s.push_str("\\u003e"),
            '&' => s.push_str("\\u0026"),
            c if (c as u32) < 0x20 => {
                let n = c as u32;
                s.push_str("\\u00");
                s.push(HEX[((n >> 4) & 0xf) as usize] as char);
                s.push(HEX[(n & 0xf) as usize] as char);
            }
            c => s.push(c),
        }
    }
    s.push('"');
}
