use crate::terminal::TerminalId;

const UNTRUSTED_LEGACY_ORIGIN_PREFIX: &str = "legacy-origin:";

pub(crate) fn legacy_origin_workspace_id(identity_key: &str) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;

    let digest = Sha256::digest(identity_key.as_bytes());
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing a digest to a string cannot fail");
    }
    format!("legacy-origin-v2:{encoded}")
}

pub(crate) fn untrusted_legacy_origin_workspace_label(id: &str) -> Option<&str> {
    id.strip_prefix(UNTRUSTED_LEGACY_ORIGIN_PREFIX)
}

/// Viewport state for a pane.
///
/// Terminal identity, cwd, labels, and agent metadata live in TerminalState.
pub struct PaneState {
    pub attached_terminal_id: TerminalId,
    /// Whether the user has seen this pane since its last state change to Idle.
    /// False = "Done" (agent finished while user was in another workspace).
    pub seen: bool,
    /// Stable id of the logical workspace this pane belonged to before it was
    /// merged into another existing workspace.
    pub origin_workspace_id: Option<String>,
    /// Cached display label of the workspace this pane originally belonged to
    /// before a cross-workspace move. This remains the fallback after the
    /// physical source workspace is closed.
    pub origin_workspace_label: Option<String>,
}

impl PaneState {
    pub fn new(attached_terminal_id: TerminalId) -> Self {
        Self {
            attached_terminal_id,
            seen: true,
            origin_workspace_id: None,
            origin_workspace_label: None,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn migrated_origin_ids_are_stable_and_do_not_alias_same_basename_paths() {
        let first = super::legacy_origin_workspace_id("cwd:/srv/one/project");
        let repeated = super::legacy_origin_workspace_id("cwd:/srv/one/project");
        let second = super::legacy_origin_workspace_id("cwd:/srv/two/project");

        assert_eq!(first, repeated);
        assert_ne!(first, second);
        assert!(first.starts_with("legacy-origin-v2:"));
        assert_eq!(
            super::untrusted_legacy_origin_workspace_label("legacy-origin:old-target"),
            Some("old-target")
        );
        assert_eq!(super::untrusted_legacy_origin_workspace_label(&first), None);
    }
}
