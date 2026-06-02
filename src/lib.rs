//! Sync, no-runtime srun (深澜) captive-portal client. Pairs a thin HTTP
//! shell over `ureq` with DNS-free captive-redirect discovery so it works
//! against any srun deployment without hand-fed `host` / `ac_id`.
//!
//! Most users want the `srunc` binary; this module is the embedding API.

mod client;
pub mod discover;
mod endpoints;
mod error;
pub mod gateway;
mod jsonp;
mod models;
pub mod profiles;
pub mod protocol;

pub use client::Client;
pub use discover::{discover, Discovered};
pub use endpoints::Endpoints;
pub use error::Error;
pub use models::Status;
pub use profiles::Profile;
pub use protocol::Credentials;
