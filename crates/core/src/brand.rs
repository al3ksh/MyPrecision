//! The name the app shows: the Dell line it runs on ("MyXPS"), or "MyStation" elsewhere.

pub const FALLBACK: &str = "MyStation";

/// Display name for an SMBIOS model string such as "Precision 5560" or "Dell XPS 15 9520".
pub fn app_name(model: Option<&str>) -> String {
    const LINES: [(&str, &str); 3] = [("precision", "MyPrecision"), ("xps", "MyXPS"), ("latitude", "MyLatitude")];
    let model = model.unwrap_or_default().trim().to_ascii_lowercase();
    let model = model.strip_prefix("dell ").unwrap_or(&model);
    LINES
        .iter()
        .find(|(line, _)| model.starts_with(line))
        .map_or(FALLBACK, |(_, name)| name)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_known_lines() {
        assert_eq!(app_name(Some("Precision 5560")), "MyPrecision");
        assert_eq!(app_name(Some("XPS 15 9520")), "MyXPS");
        assert_eq!(app_name(Some("Latitude 7440")), "MyLatitude");
    }

    #[test]
    fn ignores_dell_prefix_case_and_padding() {
        assert_eq!(app_name(Some("  Dell Precision 7680 ")), "MyPrecision");
        assert_eq!(app_name(Some("XPS13 9350")), "MyXPS");
        assert_eq!(app_name(Some("LATITUDE 5440")), "MyLatitude");
    }

    #[test]
    fn falls_back_for_other_or_missing_models() {
        assert_eq!(app_name(Some("Inspiron 16 5630")), FALLBACK);
        assert_eq!(app_name(Some("Pro Max 16")), FALLBACK);
        assert_eq!(app_name(Some("")), FALLBACK);
        assert_eq!(app_name(None), FALLBACK);
        // A line name inside another word is not a match.
        assert_eq!(app_name(Some("Imprecision 1")), FALLBACK);
    }
}
