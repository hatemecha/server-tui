//! Shared helpers for pure diagnostic rules.

pub(crate) fn sanitize_id(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::sanitize_id;

    #[test]
    fn sanitize_id_replaces_unsafe_chars() {
        assert_eq!(sanitize_id("evil/unit\u{1b}.service"), "evil_unit_.service");
        assert_eq!(sanitize_id("ok-unit_1.service"), "ok-unit_1.service");
        assert_eq!(sanitize_id("a b"), "a_b");
        assert_eq!(sanitize_id("/dev/sdb"), "_dev_sdb");
    }
}
