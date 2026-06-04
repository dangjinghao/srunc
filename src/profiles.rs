//! Deployment profiles: pre-canned `host` + `ac_id` for known srun
//! installations, so users don't have to look those up by hand.
//!
//! srun (深澜) is deployed across Chinese universities, banks, enterprises,
//! and ISPs (see <https://www.srun.com/> for the vendor's customer list).
//! Each deployment is one entry in [`PROFILES`].
//!
//! Deliberately a tiny in-binary registry, not a remote JSON: srun
//! deployments don't change often, and a network dependency for a settings
//! lookup would add supply-chain surface for nothing.
//!
//! `host` and `ac_id` are [`Option`] because we know more srun customers
//! exist than we have verified `(host, ac_id)` pairs for. A profile with
//! `None` for either is **label-only**: picking it in LuCI surfaces the
//! school's name but still falls through to captive-portal auto-discovery
//! at runtime — no risk of pinning a wrong default. When someone verifies
//! a school's real endpoint, the entry gets upgraded to `Some(...)` in a
//! PR; UIs pick up the change automatically.
//!
//! The CLI exposes the table via `srunc profiles list --json`; LuCI on
//! OpenWrt calls that through `ubus -> rpcd` to populate its dropdown.

/// One known srun deployment. `host` / `ac_id` are `Option` because the
/// entry can be a verified preset OR a label-only stub awaiting verification
/// (see module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Profile {
    /// Stable kebab-case identifier; what users pass to `--profile`.
    pub id: &'static str,
    /// Human-readable label for UIs (English + native script if useful).
    pub label: &'static str,
    /// Verified portal host, or `None` if unknown (use auto-discovery).
    pub host: Option<&'static str>,
    /// Verified access-controller id, or `None` if unknown.
    pub ac_id: Option<&'static str>,
}

/// Known deployments. Append-only; keep alphabetical by `id`. Only commit
/// `Some(...)` for `host` / `ac_id` you have **actually verified on the
/// network** — unverified pinned values silently break login and look like
/// an srunc bug. Label-only entries (`None, None`) are honest and safe.
pub const PROFILES: &[Profile] = &[
    Profile {
        id: "bit",
        label: "Beijing Institute of Technology · 北京理工大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "bnu",
        label: "Beijing Normal University · 北京师范大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "bsu",
        label: "Beijing Sport University · 北京体育大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "cug",
        label: "China University of Geosciences · 中国地质大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "ecnu",
        label: "East China Normal University · 华东师范大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "ecust",
        label: "East China University of Science and Technology · 华东理工大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "fudan",
        label: "Fudan University · 复旦大学",
        host: Some("wlrz.fudan.edu.cn"),
        ac_id: None,
    },
    Profile {
        id: "hainanu",
        label: "Hainan University · 海南大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "hit",
        label: "Harbin Institute of Technology · 哈尔滨工业大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "hnu",
        label: "Hunan University · 湖南大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "hzau",
        label: "Huazhong Agricultural University · 华中农业大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "imu",
        label: "Inner Mongolia University · 内蒙古大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "lzu",
        label: "Lanzhou University · 兰州大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "neu",
        label: "Northeastern University · 东北大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "nwpu",
        label: "Northwestern Polytechnical University · 西北工业大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "ruc",
        label: "Renmin University of China · 中国人民大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "sdu",
        label: "Shandong University · 山东大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "sustech",
        label: "Southern University of Science and Technology · 南方科技大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "tju",
        label: "Tianjin University · 天津大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "tongji",
        label: "Tongji University · 同济大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "tsinghua",
        label: "Tsinghua University · 清华大学",
        host: Some("auth4.tsinghua.edu.cn"),
        ac_id: Some("1"),
    },
    Profile {
        id: "uestc",
        label: "University of Electronic Science and Technology of China · 电子科技大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "xjtu",
        label: "Xi'an Jiaotong University · 西安交通大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "xju",
        label: "Xinjiang University · 新疆大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "xmu",
        label: "Xiamen University · 厦门大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "ynu",
        label: "Yunnan University · 云南大学",
        host: None,
        ac_id: None,
    },
    Profile {
        id: "zhejiang",
        label: "Zhejiang University · 浙江大学",
        host: Some("net.zju.edu.cn"),
        ac_id: Some("3"),
    },
];

/// Look up a profile by its stable id. Case-sensitive.
pub fn by_id(id: &str) -> Option<&'static Profile> {
    PROFILES.iter().find(|p| p.id == id)
}

/// Hosts whose `rad_user_info` response shape we've verified end-to-end —
/// `Client::status` is allowed to surface the verified-schema extras
/// (`Status::details`) only for these. Add a host here when you've sampled
/// a real online response and updated `StatusDetails` rendering accordingly.
const RICH_HOSTS: &[&str] = &["auth4.tsinghua.edu.cn"];

/// `true` if `rad_user_info` from this host is known to use the field names
/// we map into `StatusDetails`. Match is case-insensitive on the host.
pub fn host_supports_rich_status(host: &str) -> bool {
    RICH_HOSTS
        .iter()
        .any(|h| host.eq_ignore_ascii_case(h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ships_at_least_twenty_profiles() {
        // Catches accidental shrink of the registry back to a single entry.
        assert!(
            PROFILES.len() >= 20,
            "expected ≥20 profiles, got {}",
            PROFILES.len()
        );
    }

    #[test]
    fn tsinghua_is_fully_verified() {
        let p = by_id("tsinghua").expect("tsinghua profile is shipped");
        assert_eq!(p.host, Some("auth4.tsinghua.edu.cn"));
        assert_eq!(p.ac_id, Some("1"));
    }

    #[test]
    fn zhejiang_is_fully_verified() {
        let p = by_id("zhejiang").expect("zhejiang profile is shipped");
        assert_eq!(p.host, Some("net.zju.edu.cn"));
        assert_eq!(p.ac_id, Some("3"));
    }

    #[test]
    fn label_only_profile_resolves() {
        // Picking a label-only profile still surfaces a Profile struct —
        // resolution then falls through to captive-portal auto-discovery
        // at runtime, without pinning a wrong default.
        let p = by_id("ruc").expect("ruc profile is shipped");
        assert!(p.host.is_none() && p.ac_id.is_none());
        assert!(p.label.contains("Renmin"));
    }

    #[test]
    fn unknown_id_returns_none() {
        assert!(by_id("not-a-real-site").is_none());
        assert!(by_id("").is_none());
    }

    #[test]
    fn ids_are_unique_and_kebab_case() {
        let mut ids: Vec<&str> = PROFILES.iter().map(|p| p.id).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "profile ids must be unique");

        for id in PROFILES.iter().map(|p| p.id) {
            assert!(
                id.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
                "profile id {id:?} is not kebab-case"
            );
        }
    }

    #[test]
    fn registry_is_sorted_by_id() {
        let ids: Vec<&str> = PROFILES.iter().map(|p| p.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "PROFILES must be sorted alphabetically by id");
    }
}
