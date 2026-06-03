//! Strip a `callback(...)` JSONP wrapper and return the inner JSON.

use crate::Error;

pub fn extract(body: &str, cb: &str) -> Result<String, Error> {
    let n = cb.len();
    if body.len() < n + 2
        || !body.starts_with(cb)
        || body.as_bytes()[n] != b'('
        || !body.ends_with(')')
    {
        return Err(Error::InvalidJsonp);
    }
    Ok(body[n + 1..body.len() - 1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_payload() {
        assert_eq!(extract("cb()", "cb").unwrap(), "");
        assert_eq!(extract("C({})", "C").unwrap(), "{}");
        assert_eq!(
            extract(r#"jQuery({"key1": 1234})"#, "jQuery").unwrap(),
            r#"{"key1": 1234}"#
        );
    }

    #[test]
    fn rejects_malformed() {
        assert!(extract("C({}", "C").is_err());
        assert!(extract("C({})", "").is_err());
        assert!(extract("C({})", "Q").is_err());
        assert!(extract("", "C").is_err());
    }
}
