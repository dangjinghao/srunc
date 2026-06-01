#![no_std]
//! Pure, `no_std`-friendly core of the srun (深澜) campus-portal auth protocol.
//!
//! Everything here is a deterministic, side-effect-free function of its inputs
//! (no network, no clock, no globals). That is what makes the reverse-engineered
//! crypto trivially testable against captured browser/reference vectors — see
//! `tests/vectors.rs`, which pins the exact outputs produced by the upstream Go
//! implementation.
//!
//! The "imperative shell" (HTTP, HTML scraping, the keep-alive loop) belongs in a
//! separate crate that depends on this one; it is intentionally not here.

extern crate alloc;

mod base64;
mod digest;
mod error;
mod params;
mod xencode;

pub use base64::quirk_base64;
pub use digest::{md5_hex, sha1_hex};
pub use error::{message_for_code, PortalError};
pub use params::{build_portal_params, Action, Credentials, PortalParams};
pub use xencode::xencode;
