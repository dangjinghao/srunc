//! Sync `ureq`-based HTTP client speaking the srun portal protocol, plus
//! DNS-free captive-redirect discovery so the client works on any srun
//! deployment without hand-fed `host`/`ac_id`.

mod client;
pub mod discover;
mod endpoints;
mod error;
pub mod gateway;
mod jsonp;
mod models;
pub mod profiles;

pub use client::Client;
pub use discover::{discover, Discovered};
pub use endpoints::Endpoints;
pub use error::Error;
pub use models::Status;
pub use profiles::Profile;
