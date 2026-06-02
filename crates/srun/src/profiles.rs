//! Deployment profiles: pre-canned `host` + `ac_id` for known srun
//! installations, so users don't have to look those up by hand.
//!
//! srun (深澜) deployments are not limited to universities — banks,
//! enterprises, and ISPs run them too (see <https://www.srun.com/> for the
//! vendor's customer list), so this module is intentionally called
//! *profiles* rather than *schools*.
//!
//! This is a deliberately tiny, in-binary registry — not a remote JSON.
//! srun deployments don't change often, and a network dependency for a
//! settings lookup would be silly. Adding a profile = one PR appending to
//! [`PROFILES`] (only commit verified `host`/`ac_id` pairs).
//!
//! The CLI exposes the table via `srunc profiles list --json`; LuCI on
//! OpenWrt calls that through `ubus -> rpcd` to populate its dropdown.
//! Both sides resolve to the same data — no duplication.

/// One known srun deployment. All fields are `&'static str` so the table
/// is just a compile-time constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Profile {
    /// Stable kebab-case identifier; what users pass to `--profile`.
    pub id: &'static str,
    /// Human-readable label for UIs (English + native script if useful).
    pub label: &'static str,
    /// Default portal host (the IPv4 endpoint by convention).
    pub host: &'static str,
    /// Default access-controller id for that deployment.
    pub ac_id: &'static str,
}

/// Known deployments. Append-only; please keep alphabetical by `id`. Only
/// add entries whose `host`/`ac_id` you have verified on the network —
/// unverified guesses are worse than no entry.
pub const PROFILES: &[Profile] = &[Profile {
    id: "tsinghua",
    label: "Tsinghua University · 清华大学",
    host: "auth4.tsinghua.edu.cn",
    ac_id: "1",
}];

/// Look up a profile by its stable id. Case-sensitive.
pub fn by_id(id: &str) -> Option<&'static Profile> {
    PROFILES.iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_non_empty() {
        assert!(!PROFILES.is_empty());
    }

    #[test]
    fn tsinghua_resolves() {
        let p = by_id("tsinghua").expect("tsinghua profile is shipped");
        assert_eq!(p.host, "auth4.tsinghua.edu.cn");
        assert_eq!(p.ac_id, "1");
    }

    #[test]
    fn unknown_id_returns_none() {
        assert!(by_id("not-a-real-site").is_none());
        assert!(by_id("").is_none());
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<&str> = PROFILES.iter().map(|p| p.id).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "profile ids must be unique");
    }
}
