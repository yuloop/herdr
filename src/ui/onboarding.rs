use ratatui::layout::Rect;

pub(crate) fn onboarding_title() -> String {
    rust_i18n::t!("onboarding.brand").to_string()
}

pub(crate) fn onboarding_subtitle() -> String {
    rust_i18n::t!("onboarding.subtitle").to_string()
}

pub(crate) fn onboarding_description() -> Vec<String> {
    rust_i18n::t!("onboarding.body")
        .to_string()
        .lines()
        .map(str::to_owned)
        .collect()
}

pub(crate) const ONBOARDING_PREFIX_LABEL: &str = "ctrl+b";

pub(crate) fn onboarding_prefix_suffix() -> String {
    rust_i18n::t!("onboarding.prefix_hint").to_string()
}

pub(crate) const ONBOARDING_HELP_LABEL: &str = "?";

pub(crate) fn onboarding_help_suffix() -> String {
    rust_i18n::t!("onboarding.keybind_hint").to_string()
}

pub(crate) fn onboarding_next() -> String {
    rust_i18n::t!("onboarding.next").to_string()
}

pub(crate) fn onboarding_continue_label() -> String {
    format!(" ↵ {} ", rust_i18n::t!("onboarding.continue_btn"))
}

pub(crate) fn onboarding_welcome_continue_rect(area: Rect) -> Rect {
    super::widgets::continue_button_rect(area)
}
