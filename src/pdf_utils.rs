//! Shared PDF parsing utilities used across multiple modules.

/// Extract a string value from a PDF dictionary for a given key.
///
/// Returns `Some(String)` if the key exists and contains a valid non-empty string,
/// `None` otherwise.
pub fn extract_string_from_dict(dict: &lopdf::Dictionary, key: &[u8]) -> Option<String> {
    dict.get(key)
        .ok()
        .and_then(|v| v.as_str().ok())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .filter(|s| !s.is_empty())
}