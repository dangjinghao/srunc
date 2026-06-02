//! Typed views of the JSON the srun portal hands back.
//!
//! We only deserialize the fields we actually consume — the portal is generous
//! with extras (`srun_ver`, `sysver`, `ploy_msg`, …) that we don't need.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize)]
pub struct Status {
    pub online: bool,
    pub username: Option<String>,
    pub ip: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ChallengeResp {
    pub res: String,
    #[serde(default)]
    pub challenge: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct LoginResp {
    pub error: String,
    #[serde(default)]
    pub ecode: Option<Ecode>,
}

/// `ecode` is sometimes a string ("E2553"), sometimes a number (0 on success).
/// We only act on the `Str` variant; `Num` exists so serde can accept the
/// numeric form without failing the whole deserialization.
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub(crate) enum Ecode {
    Str(String),
    #[allow(dead_code)]
    Num(i64),
}

#[derive(Deserialize, Debug)]
pub(crate) struct UserInfoResp {
    pub error: String,
    #[serde(default)]
    pub user_name: Option<String>,
}
