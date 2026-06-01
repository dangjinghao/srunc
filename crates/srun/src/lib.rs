//! Imperative shell over [`srun_core`]: a sync `ureq`-based HTTP client that
//! speaks the srun portal protocol, plus a small `tsinghua` adapter module for
//! the site-specific `ac_id` HTML scraping.
//!
//! Design choices:
//!
//! * **Sync, no async runtime.** A single keep-alive timer doesn't justify the
//!   weight of tokio; `ureq` keeps the binary small and the code straight-line.
//! * **No global state.** The HTTP agent and endpoints live on [`Client`].
//! * **`context`-style cancellation** is the caller's job (a stop flag plus a
//!   bounded sleep in the keep-alive loop). We don't bake that into the core.

mod client;
mod endpoints;
mod error;
mod jsonp;
mod models;
pub mod tsinghua;

pub use client::Client;
pub use endpoints::Endpoints;
pub use error::Error;
pub use models::Status;
