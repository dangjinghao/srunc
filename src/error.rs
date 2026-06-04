use std::borrow::Cow;

use thiserror::Error;

/// Errors raised by the srun I/O shell. `Portal` wraps the typed
/// [`crate::protocol::PortalError`] for documented portal failures (E2553
/// wrong password, E2620 already online, …); raw transport / parse failures
/// get their own variants so callers can distinguish "network is down" from
/// "the portal rejected you".
#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(Box<ureq::Error>),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid JSONP envelope")]
    InvalidJsonp,

    #[error("could not parse IP from online-check response")]
    NoIpInResponse,

    #[error("challenge failed: {0}")]
    Challenge(String),

    #[error("portal: {0}")]
    Portal(crate::protocol::PortalError),

    #[error("ac_id not detected at {url}")]
    AcIdNotFound { url: String },
}

// ureq::Error is large; box it to keep this enum compact.
impl From<ureq::Error> for Error {
    fn from(e: ureq::Error) -> Self {
        Error::Http(Box::new(e))
    }
}

impl Error {
    /// Short, end-user-facing rendering. The `Display` impl stays as the
    /// developer-facing message; this one is for the top-level CLI error
    /// path, where a wall-of-text chain helps no one. With `-v` the caller
    /// should still emit the full chain.
    pub fn user_message(&self) -> Cow<'static, str> {
        match self {
            Error::Http(e) => http_user_message(e),
            Error::Io(e) => format!("local I/O error: {e}").into(),
            Error::Json(_) | Error::InvalidJsonp => {
                "portal returned a malformed response".into()
            }
            Error::NoIpInResponse => {
                "portal didn't include your IP in its response — are you on the captive network?"
                    .into()
            }
            Error::Challenge(s) => format!("portal rejected the challenge handshake: {s}").into(),
            Error::Portal(p) => portal_user_message(p),
            Error::AcIdNotFound { url } => format!(
                "couldn't detect ac_id from {url} — pass --ac-id or pick a profile with --profile <id>"
            )
            .into(),
        }
    }
}

fn portal_user_message(p: &crate::protocol::PortalError) -> Cow<'static, str> {
    use crate::protocol::PortalError;
    match p {
        PortalError::WrongPassword => "wrong password (E2553)".into(),
        PortalError::AlreadyOnline => "already online (E2620) — call `logout` first".into(),
        PortalError::UserNotExist => "user not found (E2531) — check the username".into(),
        PortalError::Known { code, message } => format!("portal: {message} ({code})").into(),
        PortalError::Unknown(code) => {
            format!("portal: unknown error ({code}) — try -v for more detail").into()
        }
    }
}

/// ureq 2.x's `Error::Transport` doesn't expose enough structure to pattern
/// match on cleanly, so we sniff the rendered string for the few common
/// failure modes users actually hit. The full message is preserved under
/// `Display`, so `-v` still shows it.
fn http_user_message(e: &ureq::Error) -> Cow<'static, str> {
    match e {
        ureq::Error::Status(code, _) => format!("portal returned HTTP {code}").into(),
        ureq::Error::Transport(t) => {
            let s = t.to_string().to_ascii_lowercase();
            if s.contains("connection refused") {
                "portal unreachable (connection refused) — is the gateway up?".into()
            } else if s.contains("timed out") || s.contains("timeout") {
                "portal connection timed out — check Wi-Fi / VPN".into()
            } else if s.contains("dns")
                || s.contains("failed to lookup")
                || s.contains("name resolution")
            {
                "portal DNS lookup failed — Wi-Fi may not yet be associated".into()
            } else if s.contains("network is unreachable") || s.contains("no route") {
                "no route to portal — check Wi-Fi association".into()
            } else if s.contains("certificate") || s.contains("tls") || s.contains("ssl") {
                "TLS handshake with portal failed (try --insecure on trusted LANs)".into()
            } else {
                format!("network error reaching portal ({t})").into()
            }
        }
    }
}
