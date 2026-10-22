//! Generic captive-portal discovery for srun deployments.
//!
//! One plaintext HTTP probe of the gateway IP — DNS-free — returns a redirect
//! that encodes both the auth host (`Location`) and the segment-correct
//! `ac_id` (`index_<N>.html` / `?ac_id=<N>`). Two pieces of config from one
//! request, works on any srun campus, survives routers that have no DNS
//! until after authentication.
//!
//! Probing the gateway IP (e.g. `http://101.6.40.1/`) yields the
//! segment-correct ac_id; probing the auth host's root often yields a
//! generic `ac_id=1` that doesn't match the network segment.

use std::time::Duration;

use ureq::AgentBuilder;

use crate::error::Error;

/// Result of one discovery probe. Either field may be `None` if it couldn't be
/// parsed — callers compose this with explicit config and other fallbacks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Discovered {
    pub host: Option<String>,
    pub ac_id: Option<String>,
}

/// Probe `http://<probe_target>/` and parse the srun redirect it returns.
///
/// `probe_target` should be reachable WITHOUT DNS (i.e. an IP, typically the
/// default IPv4 gateway from [`crate::gateway::default_gateway_v4`], or an
/// explicit `--probe-ip` value on platforms where that lookup is unavailable),
/// so this works pre-auth on routers where DNS is broken.
pub fn discover(probe_target: &str, timeout: Duration) -> Result<Discovered, Error> {
    let agent = AgentBuilder::new().timeout(timeout).build();
    let probe_url = format!("http://{probe_target}/");
    let resp = agent.get(&probe_url).call()?;
    let landed = resp.get_url().to_string();
    let body = resp.into_string()?;
    // If `landed` differs from what we probed, an HTTP 302 was followed and
    // `landed` IS the portal URL — trust it for the host. Otherwise (JS or
    // meta refresh, no HTTP redirect actually taken), `landed` is just the
    // probe IP and we should only mine the body.
    let redirect = (landed != probe_url).then_some(landed.as_str());
    Ok(parse(&body, redirect))
}

/// Extract `{host, ac_id}` from a captured response body and (optionally) the
/// URL we landed at after an HTTP redirect. Pure — separated from the I/O so
/// it's unit-testable.
///
/// Pass `Some(landed_url)` only when an HTTP 3xx was actually followed (so
/// `landed_url` is the authoritative portal URL); pass `None` when the only
/// indirection was a JS / `<meta>` redirect inside `body`.
pub fn parse(body: &str, redirect_url: Option<&str>) -> Discovered {
    Discovered {
        host: redirect_url
            .and_then(extract_host)
            .or_else(|| extract_host(body)),
        ac_id: redirect_url
            .and_then(extract_ac_id)
            .or_else(|| extract_ac_id(body)),
    }
}

/// Find the first `http(s)://<host>` in `text` and return the `<host>` (with
/// port if present). Stops at `/`, whitespace, `"`, `'`, `?`, `#`, `<`, `>`, `)`.
fn extract_host(text: &str) -> Option<String> {
    let mut search = text;
    loop {
        let sep = search.find("://")?;
        let before = &search[..sep];
        let scheme_len = if before.ends_with("https") {
            5
        } else if before.ends_with("http") {
            4
        } else {
            // Not http(s); advance past this "://" and keep looking.
            search = &search[sep + 3..];
            continue;
        };
        let host_start = sep + 3;
        let after = &search[host_start..];
        let end = after
            .find(|c: char| {
                matches!(c, '/' | '"' | '\'' | '?' | '#' | '<' | '>' | ')' | '\\')
                    || c.is_whitespace()
            })
            .unwrap_or(after.len());
        let host = &after[..end];
        if !host.is_empty() && host.contains('.') {
            // Sanity-check: must look like a hostname (has a dot).
            return Some(host.to_string());
        }
        // Skip past this match and continue.
        search = &search[host_start + end..];
        // Avoid infinite loop on empty match.
        let _ = scheme_len;
    }
}

/// Find `ac_id=<digits>` or `index_<digits>` and return the digits.
fn extract_ac_id(text: &str) -> Option<String> {
    // Search every occurrence, take the first one whose digit-run is non-empty.
    for pat in ["ac_id=", "index_"] {
        let mut from = 0;
        while let Some(rel) = text[from..].find(pat) {
            let abs = from + rel + pat.len();
            let rest = &text[abs..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                return Some(digits);
            }
            from = abs;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_from_url_with_path() {
        assert_eq!(
            extract_host("http://auth4.tsinghua.edu.cn/index_169.html"),
            Some("auth4.tsinghua.edu.cn".into())
        );
    }

    #[test]
    fn host_keeps_port() {
        assert_eq!(
            extract_host("https://portal.example.com:8443/login"),
            Some("portal.example.com:8443".into())
        );
    }

    #[test]
    fn host_in_js_redirect() {
        let body =
            r#"<script>location.href="http://auth4.tsinghua.edu.cn/index_169.html"</script>"#;
        assert_eq!(extract_host(body), Some("auth4.tsinghua.edu.cn".into()));
    }

    #[test]
    fn host_in_meta_refresh() {
        let body =
            r#"<meta http-equiv="refresh" content="0;url=http://portal.example.cn/index_42.html">"#;
        assert_eq!(extract_host(body), Some("portal.example.cn".into()));
    }

    #[test]
    fn host_skips_dotless_segments() {
        // `://something` without a dot shouldn't be mistaken for a host.
        assert_eq!(
            extract_host("scheme://localhost\nthen http://real.host/"),
            Some("real.host".into())
        );
    }

    #[test]
    fn host_none_when_missing() {
        assert_eq!(extract_host("no urls here"), None);
        assert_eq!(extract_host("ftp://files.example.com/"), None);
    }

    #[test]
    fn ac_id_from_index_html() {
        assert_eq!(
            extract_ac_id("http://auth4.tsinghua.edu.cn/index_169.html"),
            Some("169".into())
        );
    }

    #[test]
    fn ac_id_from_query() {
        assert_eq!(
            extract_ac_id("/srun_portal_pc?ac_id=42&theme=pro"),
            Some("42".into())
        );
    }

    #[test]
    fn ac_id_prefers_first_real_match() {
        // An `ac_id=` with no digits should NOT shadow a later real match.
        assert_eq!(extract_ac_id("ac_id=&junk ac_id=7"), Some("7".into()));
    }

    #[test]
    fn ac_id_none_when_missing() {
        assert_eq!(extract_ac_id("nothing of interest"), None);
    }

    #[test]
    fn parse_tsinghua_gateway_js_redirect() {
        // The shape we actually saw from `GET http://101.6.40.1/` on Tsinghua.
        // No HTTP 302 was followed (`landed` == probe URL), so we pass None
        // for `redirect_url` and only mine the body.
        let body = r#"<html>
<head>
<script type="text/javascript">location.href="http://auth4.tsinghua.edu.cn/index_169.html"</script>
</head>
<body>
</body>
</html>"#;
        let got = parse(body, None);
        assert_eq!(got.host.as_deref(), Some("auth4.tsinghua.edu.cn"));
        assert_eq!(got.ac_id.as_deref(), Some("169"));
    }

    #[test]
    fn parse_landed_url_takes_precedence_for_host() {
        // After HTTP-302 following, the landed URL is the portal — trust it
        // over whatever is in the body.
        let landed = "http://portal.example.com/index_5.html";
        let body = "<a href=\"http://cdn.example.com/style.css\">x</a>";
        let got = parse(body, Some(landed));
        assert_eq!(got.host.as_deref(), Some("portal.example.com"));
        assert_eq!(got.ac_id.as_deref(), Some("5"));
    }

    #[test]
    fn parse_meta_refresh_relative_url_keeps_ac_id() {
        // Some portals 302 you to the portal, then the page itself has a
        // relative meta refresh — landed is the portal URL.
        let landed = "http://portal.example.com/srun_portal_pc?ac_id=23&theme=pro";
        let body = r#"<meta http-equiv="refresh" content="0;url=/srun_portal_pc?ac_id=23">"#;
        let got = parse(body, Some(landed));
        assert_eq!(got.host.as_deref(), Some("portal.example.com"));
        assert_eq!(got.ac_id.as_deref(), Some("23"));
    }
}
