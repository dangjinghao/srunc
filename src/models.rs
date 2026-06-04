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
    /// What `resolve_target` actually picked — handy for the LuCI dashboard
    /// to show without log-scraping. Populated by the CLI's status command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_ac_id: Option<String>,
    /// Populated only when the portal host is on the verified-schema list.
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
    /// One entry per device the portal believes is logged in on this
    /// account, parsed out of `online_device_detail`'s embedded JSON.
    /// `None` (not `Some(vec![])`) when the response carried no list at
    /// all; `Some(vec![])` if the field was present but empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devices: Option<Vec<DeviceEntry>>,
}

/// One row in `online_device_detail`. The portal serializes this as a JSON
/// *string* (string-encoded JSON) inside the outer response — see
/// `parse_devices`. We keep MAC and `class_name` separate because some
/// deployments stuff the MAC into `class_name` and leave `mac` empty.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DeviceEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip6: Option<String>,
    /// Some installs put the MAC in `class_name`; others use a `mac` key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rad_online_id: Option<String>,
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
    /// Server emits this as a *string* whose value is a JSON object keyed
    /// by `rad_online_id`. Parsed lazily by `parse_devices` so the outer
    /// deserialization never fails on a deployment that ships a different
    /// shape here.
    #[serde(default)]
    pub online_device_detail: Option<String>,
}

impl UserInfoResp {
    /// Materialize the verified-schema extras into a `StatusDetails`. Caller
    /// is responsible for only invoking this on hosts where the schema is
    /// known to match.
    pub(crate) fn into_details(self) -> StatusDetails {
        let devices = parse_devices(self.online_device_detail.as_deref());
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
            devices,
        }
    }
}

/// Parse the nested JSON-string carried in `online_device_detail`. THU emits
/// `{"<rad_id>": {...}, "<rad_id>": {...}}` (keyed by `rad_online_id`); we
/// flatten the values into a vector since the key is duplicated inside each
/// row. Returns `None` for the missing / empty / unparseable cases — UIs
/// should treat that as "deployment doesn't expose a device list" and just
/// not render the table.
fn parse_devices(raw: Option<&str>) -> Option<Vec<DeviceEntry>> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    let map: std::collections::BTreeMap<String, DeviceEntry> =
        serde_json::from_str(raw).ok()?;
    Some(map.into_values().collect())
}

fn nonempty(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_devices_round_trip_thu_sample() {
        // Minimal slice mirroring THU's shape.
        let raw = r#"{"123":{"class_name":"AA-BB-CC-DD-EE-FF","ip":"10.0.0.1","ip6":"","os_name":"","rad_online_id":"123"},"456":{"class_name":"","ip":"10.0.0.2","ip6":"::1","os_name":"","rad_online_id":"456"}}"#;
        let got = parse_devices(Some(raw)).expect("parses");
        assert_eq!(got.len(), 2);
        // BTreeMap orders by key — "123" before "456".
        assert_eq!(got[0].ip.as_deref(), Some("10.0.0.1"));
        assert_eq!(got[0].class_name.as_deref(), Some("AA-BB-CC-DD-EE-FF"));
        assert_eq!(got[1].ip6.as_deref(), Some("::1"));
    }

    #[test]
    fn parse_devices_handles_missing_and_garbage() {
        assert!(parse_devices(None).is_none());
        assert!(parse_devices(Some("")).is_none());
        assert!(parse_devices(Some("not json at all")).is_none());
    }
}
