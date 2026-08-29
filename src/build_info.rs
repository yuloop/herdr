//! Build identity helpers.

pub const BASE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn channel() -> &'static str {
    non_empty(option_env!("HERDR_BUILD_CHANNEL")).unwrap_or("stable")
}

pub fn build_id() -> Option<&'static str> {
    non_empty(option_env!("HERDR_BUILD_ID"))
}

pub fn version() -> String {
    match channel() {
        "stable" => BASE_VERSION.to_string(),
        channel => match build_id() {
            Some(build_id) => format!("{BASE_VERSION}-{channel}.{build_id}"),
            None => format!("{BASE_VERSION}-{channel}"),
        },
    }
}

pub fn is_preview() -> bool {
    channel() == "preview"
}

/// Whether this binary may discover and install releases from Herdr's official
/// stable/preview update service.
///
/// Distribution-specific channels (for example, `deploy`) are updated by the
/// workflow that produced them so official releases cannot overwrite local
/// patches.
pub fn official_updates_enabled() -> bool {
    official_updates_enabled_for_channel(channel())
}

fn official_updates_enabled_for_channel(channel: &str) -> bool {
    matches!(channel, "stable" | "preview")
}

fn non_empty(value: Option<&'static str>) -> Option<&'static str> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn stable_version_defaults_to_cargo_version() {
        assert!(!super::version().is_empty());
    }

    #[test]
    fn official_channels_use_the_official_update_service() {
        assert!(super::official_updates_enabled_for_channel("stable"));
        assert!(super::official_updates_enabled_for_channel("preview"));
    }

    #[test]
    fn distribution_channels_do_not_use_the_official_update_service() {
        assert!(!super::official_updates_enabled_for_channel("deploy"));
        assert!(!super::official_updates_enabled_for_channel("nightly"));
    }
}
