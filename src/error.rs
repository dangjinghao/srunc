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
