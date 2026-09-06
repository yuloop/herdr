pub fn apply_locale(language: &str) {
    let locale = normalize_language(language);
    rust_i18n::set_locale(locale);
}

fn normalize_language(language: &str) -> &'static str {
    let normalized = language.trim().to_lowercase();
    if normalized.starts_with("zh") {
        "zh"
    } else {
        "en"
    }
}
