//! Typed views of the JSON the srun portal hands back.
//!
//! `UserInfoResp` is permissive: every field beyond `error` is `Option<_>` so
//! the same struct can deserialize responses from any srun deployment without
//! tripping on absent or unexpected fields. The richer fields only get
//! surfaced in `Status::details` for deployments whose schema we've verified
//! (see `profiles::host_supports_rich_status`).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize)]
pub struct Status {
    pub online: bool,
    pub username: Option<String>,
    pub ip: Option<String>,
    /// Populated only when the portal host is on the verified-schema list
    /// (currently just Tsinghua's `auth4.tsinghua.edu.cn`). `None` means
    /// "we got a response but don't trust the field semantics here" — UIs
    /// should suppress detail rows in that case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<StatusDetails>,
}

/// Verified-schema extras from `rad_user_info`. All fields are `Option`
/// because even within one deployment some are empty (e.g. THU students
/// without a phone number on file).
#[derive(Clone, Debug, Default, Serialize)]
pub struct StatusDetails {
    // --- plan / billing ---
    /// `products_name` — human label for the data plan (e.g. "特殊学生").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    /// `billing_name` — billing-policy label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing: Option<String>,
    /// `user_balance`. Unit is deployment-defined and not confirmed for THU
    /// (all observed values are 0); exposed as-is for downstream rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_balance: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_balance: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_charge: Option<i64>,

    // --- traffic (bytes, cumulative for the current billing period) ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sum_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_in: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_out: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remain_bytes: Option<u64>,

    // --- time (seconds, cumulative) ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sum_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remain_seconds: Option<u64>,
    /// Unix timestamp of the most recent server-side keepalive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keepalive_time: Option<u64>,
    /// Unix timestamp the account / session was added.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_time: Option<u64>,

    // --- devices / identity ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub online_device_total: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_mac: Option<String>,
    /// `online_ip` — the IPv4 the portal attributes the session to. May
    /// differ from `Status::ip` (which we extract from the online-check
    /// HTML) on dual-stack setups.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub online_ip: Option<String>,
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

#[derive(Deserialize, Debug, Default)]
pub(crate) struct UserInfoResp {
    pub error: String,
    #[serde(default)]
    pub user_name: Option<String>,

    // Verified-schema extras. Every field below is best-effort: missing in
    // the response → `None`; we only consume them when the caller has flagged
    // the host as having a verified schema.
    #[serde(default)]
    pub products_name: Option<String>,
    #[serde(default)]
    pub billing_name: Option<String>,
    #[serde(default)]
    pub user_balance: Option<i64>,
    #[serde(default)]
    pub wallet_balance: Option<i64>,
    #[serde(default)]
    pub user_charge: Option<i64>,
    #[serde(default)]
    pub sum_bytes: Option<u64>,
    #[serde(default)]
    pub bytes_in: Option<u64>,
    #[serde(default)]
    pub bytes_out: Option<u64>,
    #[serde(default)]
    pub remain_bytes: Option<u64>,
    #[serde(default)]
    pub sum_seconds: Option<u64>,
    #[serde(default)]
    pub remain_seconds: Option<u64>,
    #[serde(default)]
    pub keepalive_time: Option<u64>,
    #[serde(default)]
    pub add_time: Option<u64>,
    /// Server sends this as a string ("3"), not a number — keep as String
    /// here, parse at the consumer.
    #[serde(default)]
    pub online_device_total: Option<String>,
    #[serde(default)]
    pub user_mac: Option<String>,
    #[serde(default)]
    pub online_ip: Option<String>,
}

impl UserInfoResp {
    /// Materialize the verified-schema extras into a `StatusDetails`. Caller
    /// is responsible for only invoking this on hosts where the schema is
    /// known to match.
    pub(crate) fn into_details(self) -> StatusDetails {
        StatusDetails {
            plan: nonempty(self.products_name),
            billing: nonempty(self.billing_name),
            user_balance: self.user_balance,
            wallet_balance: self.wallet_balance,
            user_charge: self.user_charge,
            sum_bytes: self.sum_bytes,
            bytes_in: self.bytes_in,
            bytes_out: self.bytes_out,
            remain_bytes: self.remain_bytes,
            sum_seconds: self.sum_seconds,
            remain_seconds: self.remain_seconds,
            keepalive_time: self.keepalive_time,
            add_time: self.add_time,
            online_device_total: self
                .online_device_total
                .as_deref()
                .and_then(|s| s.trim().parse().ok()),
            user_mac: nonempty(self.user_mac),
            online_ip: nonempty(self.online_ip),
        }
    }
}

fn nonempty(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.is_empty())
}
