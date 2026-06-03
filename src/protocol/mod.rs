//! Pure, side-effect-free core of the srun (深澜) auth protocol — XEncode,
//! quirk-base64, digests, and login/logout parameter assembly. Byte output is
//! pinned by the golden vectors in `tests/protocol_vectors.rs`.

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
