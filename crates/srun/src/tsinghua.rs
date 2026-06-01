//! Tsinghua-specific quirks. Kept out of the generic protocol code so the
//! `Client` stays portable to other srun deployments.

use crate::{Client, Error};

/// Scrape the public-side login page for the current `ac_id` (control-policy id).
/// Looks for either `ac_id=NUM` or `index_NUM` in the response body and the
/// landed URL after redirects.
pub fn detect_ac_id(client: &Client, v6: bool) -> Result<String, Error> {
    let url = if v6 {
        "http://mirrors6.tuna.tsinghua.edu.cn/"
    } else {
        "http://login.tsinghua.edu.cn/index_1.html"
    };
    let resp = client.agent.get(url).call()?;
    let landed = resp.get_url().to_string();
    let body = resp.into_string()?;
    detect_from(&body)
        .or_else(|| detect_from(&landed))
        .ok_or(Error::AcIdNotFound { url: url.into() })
}

fn detect_from(text: &str) -> Option<String> {
    for pat in ["ac_id=", "index_"] {
        if let Some(idx) = text.find(pat) {
            let rest = &text[idx + pat.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                return Some(digits);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_ac_id_var() {
        assert_eq!(detect_from(r#"... ac_id=42 ..."#), Some("42".into()));
    }

    #[test]
    fn finds_index_filename() {
        assert_eq!(detect_from("/index_7.html"), Some("7".into()));
    }

    #[test]
    fn returns_none_when_missing() {
        assert_eq!(detect_from("nothing of interest"), None);
        assert_eq!(detect_from("ac_id="), None); // no digits
    }
}
