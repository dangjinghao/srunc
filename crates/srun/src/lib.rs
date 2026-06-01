//! Imperative shell over [`srun_core`]: a sync `ureq`-based HTTP client that
//! speaks the srun portal protocol, plus zero-config discovery so the client
//! works on any srun deployment without hand-fed `host`/`ac_id`.
//!
//! Design choices:
//!
//! * **Sync, no async runtime.** A single keep-alive timer doesn't justify the
//!   weight of tokio; `ureq` keeps the binary small and the code straight-line.
//! * **No global state.** The HTTP agent and endpoints live on [`Client`].
//! * **`context`-style cancellation** is the caller's job (a stop flag plus a
//!   bounded sleep in the keep-alive loop). We don't bake that into the core.
//! * **Generic over deployment.** The protocol code in [`Client`] never names
//!   a specific campus; per-deployment defaults live in [`tsinghua`] only as
//!   convenience, and [`discover`]/[`gateway`] derive the host + `ac_id` from
//!   the captive-portal redirect itself for fully automatic configuration.

mod client;
pub mod discover;
mod endpoints;
mod error;
pub mod gateway;
mod jsonp;
mod models;
pub mod tsinghua;

pub use client::Client;
pub use discover::{discover, Discovered};
pub use endpoints::Endpoints;
pub use error::Error;
pub use models::Status;
