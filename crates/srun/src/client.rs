//! The HTTP client.

use std::time::Duration;

use ureq::{Agent, AgentBuilder};

use srun_core::{build_portal_params, Action, Credentials, PortalError, PortalParams};

use crate::endpoints::Endpoints;
use crate::error::Error;
use crate::jsonp;
use crate::models::{ChallengeResp, Ecode, LoginResp, Status, UserInfoResp};

const CB: &str = "C_a_l_l_b_a_c_k";

pub struct Client {
    pub agent: Agent,
    pub endpoints: Endpoints,
}

impl Client {
    pub fn new(endpoints: Endpoints, timeout: Duration) -> Self {
        let agent = AgentBuilder::new().timeout(timeout).build();
        Self { agent, endpoints }
    }

    // ---- low-level ----

    fn get_jsonp(&self, base: &str, params: &[(&str, &str)]) -> Result<String, Error> {
        let mut all: Vec<(&str, &str)> = params.to_vec();
        all.push(("callback", CB));
        let url = format!("{base}?{}", encode_query(&all));
        let body = self.agent.get(&url).call()?.into_string()?;
        jsonp::extract(&body, CB)
    }

    // ---- high-level ----

    /// Are we online? Returns the user_name and public IP if so.
    pub fn status(&self, ac_id: &str) -> Result<Status, Error> {
        let url = format!(
            "{}?{}",
            self.endpoints.online_check,
            encode_query(&[("ac_id", ac_id)])
        );
        let body = self.agent.get(&url).call()?.into_string()?;
        let ip = extract_ip(&body).ok_or(Error::NoIpInResponse)?;

        let raw = self.get_jsonp(&self.endpoints.user_info, &[("ip", &ip)])?;
        let info: UserInfoResp = serde_json::from_str(&raw)?;
        Ok(Status {
            online: info.error == "ok",
            username: info.user_name,
            ip: Some(ip),
        })
    }

    pub fn challenge(&self, username: &str, ip: &str) -> Result<String, Error> {
        let raw = self.get_jsonp(
            &self.endpoints.challenge,
            &[("username", username), ("ip", ip), ("double_stack", "1")],
        )?;
        let r: ChallengeResp = serde_json::from_str(&raw)?;
        if r.res != "ok" {
            return Err(Error::Challenge(r.res));
        }
        r.challenge
            .ok_or_else(|| Error::Challenge("no `challenge` field in response".into()))
    }

    pub fn login(&self, creds: &Credentials, ac_id: &str, ip: &str) -> Result<(), Error> {
        let token = self.challenge(creds.username, ip)?;
        let p = build_portal_params(creds, &token, Action::Login, ip, ac_id);
        self.submit(&p)
    }

    pub fn logout(&self, username: &str, ac_id: &str, ip: &str) -> Result<(), Error> {
        let token = self.challenge(username, ip)?;
        // Logout ignores the password field; pass an empty one.
        let creds = Credentials {
            username,
            password: "",
        };
        let p = build_portal_params(&creds, &token, Action::Logout, ip, ac_id);
        self.submit(&p)
    }

    fn submit(&self, p: &PortalParams) -> Result<(), Error> {
        let pairs = p.to_pairs();
        let raw = self.get_jsonp(&self.endpoints.login, &pairs)?;
        let r: LoginResp = serde_json::from_str(&raw)?;
        if r.error == "ok" {
            return Ok(());
        }
        let code = match r.ecode {
            Some(Ecode::Str(s)) if !s.is_empty() => s,
            _ => r.error.clone(),
        };
        Err(Error::Portal(PortalError::from_code(&code)))
    }
}

// ---- helpers ----

/// Percent-encode (RFC 3986 unreserved set untouched, everything else %HH).
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                let hi = b >> 4;
                let lo = b & 0x0f;
                out.push('%');
                out.push(hex_char(hi));
                out.push(hex_char(lo));
            }
        }
    }
    out
}

fn hex_char(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'A' + n - 10) as char,
        _ => '?',
    }
}

fn encode_query(pairs: &[(&str, &str)]) -> String {
    let mut s = String::new();
    for (i, (k, v)) in pairs.iter().enumerate() {
        if i > 0 {
            s.push('&');
        }
        s.push_str(&urlencode(k));
        s.push('=');
        s.push_str(&urlencode(v));
    }
    s
}

/// Find `ip<ws>:<ws>"<digits/dots>"` in the body — port of Go's regex
/// `ip\s+:\s"([0-9.]+)"`. Loops over candidate `ip` positions so spurious
/// matches inside other words don't trip us up.
fn extract_ip(body: &str) -> Option<String> {
    let mut from = 0;
    while let Some(pos) = body[from..].find("ip") {
        let abs = from + pos;
        let after = &body.as_bytes()[abs + 2..];
        let mut i = 0;
        while i < after.len() && after[i].is_ascii_whitespace() {
            i += 1;
        }
        if i == 0 || i >= after.len() || after[i] != b':' {
            from = abs + 1;
            continue;
        }
        i += 1;
        while i < after.len() && after[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= after.len() || after[i] != b'"' {
            from = abs + 1;
            continue;
        }
        i += 1;
        let start = i;
        while i < after.len() && (after[i].is_ascii_digit() || after[i] == b'.') {
            i += 1;
        }
        if i == start {
            from = abs + 1;
            continue;
        }
        return std::str::from_utf8(&after[start..i]).ok().map(String::from);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_encodes_correctly() {
        assert_eq!(urlencode("hello world"), "hello%20world");
        assert_eq!(urlencode("a/b+c"), "a%2Fb%2Bc");
        assert_eq!(urlencode("-_.~AZaz09"), "-_.~AZaz09");
    }

    #[test]
    fn extract_ip_finds_value() {
        // The srun /srun_portal_pc response uses a JS-literal-ish form with
        // unquoted keys, which is exactly what the Go regex `ip\s+:\s"..."`
        // is shaped for.
        assert_eq!(
            extract_ip(r#" foo  ip : "166.111.4.100" bar "#),
            Some("166.111.4.100".to_string())
        );
        assert_eq!(
            extract_ip("online,\n  ip : \"1.2.3.4\",\n"),
            Some("1.2.3.4".to_string())
        );
    }

    #[test]
    fn extract_ip_skips_non_match() {
        assert_eq!(extract_ip("no ip address here"), None);
        assert_eq!(extract_ip(r#"ip:noquotes"#), None);
    }
}
