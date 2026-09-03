use crate::terminal::TerminalId;

/// Viewport state for a pane.
///
/// Terminal identity, cwd, labels, and agent metadata live in TerminalState.
pub struct PaneState {
    pub attached_terminal_id: TerminalId,
    pub seen: bool,
    pub right_click_passthrough: bool,
    pub origin_workspace_id: Option<String>,
    pub origin_workspace_label: Option<String>,
}

impl PaneState {
    pub fn new(attached_terminal_id: TerminalId) -> Self {
        Self {
            attached_terminal_id,
            seen: true,
            right_click_passthrough: false,
            origin_workspace_id: None,
            origin_workspace_label: None,
        }
    }
}
