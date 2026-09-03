use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    config::{ActionKeybinds, IndexedKeybind, Keybinds},
    input::TerminalKey,
};

pub(crate) type KeybindHelpEntry = (String, Cow<'static, str>);
pub(crate) type KeybindHelpGroup = (Cow<'static, str>, Vec<KeybindHelpEntry>);

pub(crate) fn keybind_help_text_char(key: &TerminalKey) -> Option<char> {
    if !key.modifiers.difference(KeyModifiers::SHIFT).is_empty() {
        return None;
    }
    if let Some(character) = key.shifted_codepoint.and_then(char::from_u32) {
        return Some(character);
    }
    let KeyCode::Char(character) = key.code else {
        return None;
    };
    Some(character)
}

fn entry(key: impl Into<String>, label: Cow<'static, str>) -> KeybindHelpEntry {
    (key.into(), label)
}

fn binding_label(bindings: &ActionKeybinds) -> String {
    bindings
        .label()
        .unwrap_or_else(|| rust_i18n::t!("common.unset").to_string())
}

fn indexed_label(bindings: &[IndexedKeybind]) -> String {
    if bindings.is_empty() {
        return rust_i18n::t!("common.unset").to_string();
    }
    let mut parts = Vec::new();
    let mut index = 0;
    while index < bindings.len() {
        if let Some(prefix) = indexed_range_prefix(&bindings[index..]) {
            parts.push(format!("{prefix}1..9"));
            index += 9;
        } else {
            parts.push(bindings[index].label.clone());
            index += 1;
        }
    }
    parts.join(" / ")
}

fn indexed_range_prefix(bindings: &[IndexedKeybind]) -> Option<&str> {
    let run = bindings.get(..9)?;
    let prefix = run[0].label.strip_suffix('1')?;
    for (offset, binding) in run.iter().enumerate() {
        let digit = char::from(b'1' + offset as u8);
        if binding.label.strip_suffix(digit) != Some(prefix) {
            return None;
        }
    }
    Some(prefix)
}

pub(crate) fn keybind_help_groups(
    keybinds: &Keybinds,
    prefix: (crossterm::event::KeyCode, crossterm::event::KeyModifiers),
) -> Vec<KeybindHelpGroup> {
    let mut groups = vec![
        (
            Cow::Owned(rust_i18n::t!("keybind.group_global").to_string()),
            vec![
                entry(
                    crate::config::format_key_combo(prefix),
                    Cow::Owned(rust_i18n::t!("keybind.prefix_mode").to_string()),
                ),
                entry(
                    binding_label(&keybinds.help),
                    Cow::Owned(rust_i18n::t!("keybind.keybinds").to_string()),
                ),
                entry(
                    binding_label(&keybinds.settings),
                    Cow::Owned(rust_i18n::t!("keybind.settings").to_string()),
                ),
                entry(
                    binding_label(&keybinds.detach),
                    Cow::Owned(rust_i18n::t!("keybind.detach").to_string()),
                ),
                entry(
                    binding_label(&keybinds.reload_config),
                    Cow::Owned(rust_i18n::t!("keybind.reload_config").to_string()),
                ),
                entry(
                    binding_label(&keybinds.open_notification_target),
                    Cow::Owned(
                        rust_i18n::t!("keybind.open_notification_target").to_string(),
                    ),
                ),
            ],
        ),
        (
            Cow::Owned(rust_i18n::t!("keybind.group_navigation").to_string()),
            vec![
                entry(
                    "esc",
                    Cow::Owned(rust_i18n::t!("keybind.back").to_string()),
                ),
                entry(
                    format!(
                        "{} / {}",
                        binding_label(&keybinds.navigate.workspace_up),
                        binding_label(&keybinds.navigate.workspace_down)
                    ),
                    Cow::Owned(rust_i18n::t!("keybind.workspace_list").to_string()),
                ),
                entry(
                    format!(
                        "{} / {} / {} / {} / left / right",
                        binding_label(&keybinds.navigate.pane_left),
                        binding_label(&keybinds.navigate.pane_down),
                        binding_label(&keybinds.navigate.pane_up),
                        binding_label(&keybinds.navigate.pane_right)
                    ),
                    Cow::Owned(rust_i18n::t!("keybind.move_focus").to_string()),
                ),
                entry(
                    "tab / shift+tab",
                    Cow::Owned(rust_i18n::t!("keybind.cycle_pane").to_string()),
                ),
                entry(
                    "enter",
                    Cow::Owned(rust_i18n::t!("keybind.open_workspace").to_string()),
                ),
                entry(
                    "1..9",
                    Cow::Owned(rust_i18n::t!("keybind.switch_workspace").to_string()),
                ),
            ],
        ),
        (
            Cow::Owned(rust_i18n::t!("keybind.group_workspaces").to_string()),
            vec![
                entry(
                    binding_label(&keybinds.workspace_picker),
                    Cow::Owned(rust_i18n::t!("keybind.workspace_navigation").to_string()),
                ),
                entry(
                    binding_label(&keybinds.goto),
                    Cow::Owned(rust_i18n::t!("keybind.session_navigator").to_string()),
                ),
                entry(
                    binding_label(&keybinds.new_workspace),
                    Cow::Owned(rust_i18n::t!("keybind.new_workspace").to_string()),
                ),
                entry(
                    binding_label(&keybinds.new_worktree),
                    Cow::Owned(rust_i18n::t!("keybind.new_worktree").to_string()),
                ),
                entry(
                    binding_label(&keybinds.open_worktree),
                    Cow::Owned(rust_i18n::t!("keybind.open_worktree").to_string()),
                ),
                entry(
                    binding_label(&keybinds.remove_worktree),
                    Cow::Owned(
                        rust_i18n::t!("keybind.delete_worktree_checkout").to_string(),
                    ),
                ),
                entry(
                    binding_label(&keybinds.rename_workspace),
                    Cow::Owned(rust_i18n::t!("keybind.rename_workspace").to_string()),
                ),
                entry(
                    binding_label(&keybinds.close_workspace),
                    Cow::Owned(rust_i18n::t!("keybind.close_workspace").to_string()),
                ),
                entry(
                    binding_label(&keybinds.previous_workspace),
                    Cow::Owned(rust_i18n::t!("keybind.previous_workspace").to_string()),
                ),
                entry(
                    binding_label(&keybinds.next_workspace),
                    Cow::Owned(rust_i18n::t!("keybind.next_workspace").to_string()),
                ),
                entry(
                    indexed_label(&keybinds.switch_workspace),
                    Cow::Owned(rust_i18n::t!("keybind.switch_workspace_n").to_string()),
                ),
                entry(
                    binding_label(&keybinds.previous_agent),
                    Cow::Owned(rust_i18n::t!("keybind.previous_agent").to_string()),
                ),
                entry(
                    binding_label(&keybinds.next_agent),
                    Cow::Owned(rust_i18n::t!("keybind.next_agent").to_string()),
                ),
                entry(
                    indexed_label(&keybinds.focus_agent),
                    Cow::Owned(rust_i18n::t!("keybind.focus_agent_n").to_string()),
                ),
                entry(
                    binding_label(&keybinds.new_tab),
                    Cow::Owned(rust_i18n::t!("keybind.new_tab").to_string()),
                ),
                entry(
                    binding_label(&keybinds.rename_tab),
                    Cow::Owned(rust_i18n::t!("keybind.rename_tab").to_string()),
                ),
                entry(
                    binding_label(&keybinds.previous_tab),
                    Cow::Owned(rust_i18n::t!("keybind.previous_tab").to_string()),
                ),
                entry(
                    binding_label(&keybinds.next_tab),
                    Cow::Owned(rust_i18n::t!("keybind.next_tab").to_string()),
                ),
                entry(
                    binding_label(&keybinds.move_tab_previous),
                    Cow::Owned(rust_i18n::t!("keybind.move_tab_left").to_string()),
                ),
                entry(
                    binding_label(&keybinds.move_tab_next),
                    Cow::Owned(rust_i18n::t!("keybind.move_tab_right").to_string()),
                ),
                entry(
                    indexed_label(&keybinds.switch_tab),
                    Cow::Owned(rust_i18n::t!("keybind.switch_tab_n").to_string()),
                ),
                entry(
                    binding_label(&keybinds.close_tab),
                    Cow::Owned(rust_i18n::t!("keybind.close_tab").to_string()),
                ),
            ],
        ),
        (
            Cow::Owned(rust_i18n::t!("keybind.group_panes").to_string()),
            vec![
                entry(
                    binding_label(&keybinds.split_vertical),
                    Cow::Owned(rust_i18n::t!("keybind.split_vertical").to_string()),
                ),
                entry(
                    binding_label(&keybinds.split_horizontal),
                    Cow::Owned(rust_i18n::t!("keybind.split_horizontal").to_string()),
                ),
                entry(
                    binding_label(&keybinds.close_pane),
                    Cow::Owned(rust_i18n::t!("keybind.close_pane").to_string()),
                ),
                entry(
                    binding_label(&keybinds.rename_pane),
                    Cow::Owned(rust_i18n::t!("keybind.rename_pane").to_string()),
                ),
                entry(
                    binding_label(&keybinds.edit_scrollback),
                    Cow::Owned(rust_i18n::t!("keybind.edit_scrollback").to_string()),
                ),
                entry(
                    binding_label(&keybinds.copy_mode),
                    Cow::Owned(rust_i18n::t!("keybind.copy_mode").to_string()),
                ),
                entry(
                    binding_label(&keybinds.zoom),
                    Cow::Owned(rust_i18n::t!("keybind.zoom_pane").to_string()),
                ),
                entry(
                    binding_label(&keybinds.resize_mode),
                    Cow::Owned(rust_i18n::t!("keybind.resize_mode").to_string()),
                ),
                entry(
                    binding_label(&keybinds.resize_pane_left),
                    Cow::Owned(rust_i18n::t!("keybind.resize_pane_left").to_string()),
                ),
                entry(
                    binding_label(&keybinds.resize_pane_down),
                    Cow::Owned(rust_i18n::t!("keybind.resize_pane_down").to_string()),
                ),
                entry(
                    binding_label(&keybinds.resize_pane_up),
                    Cow::Owned(rust_i18n::t!("keybind.resize_pane_up").to_string()),
                ),
                entry(
                    binding_label(&keybinds.resize_pane_right),
                    Cow::Owned(rust_i18n::t!("keybind.resize_pane_right").to_string()),
                ),
                entry(
                    binding_label(&keybinds.toggle_sidebar),
                    Cow::Owned(rust_i18n::t!("keybind.toggle_sidebar").to_string()),
                ),
                entry(
                    binding_label(&keybinds.focus_pane_left),
                    Cow::Owned(rust_i18n::t!("keybind.focus_pane_left").to_string()),
                ),
                entry(
                    binding_label(&keybinds.focus_pane_down),
                    Cow::Owned(rust_i18n::t!("keybind.focus_pane_down").to_string()),
                ),
                entry(
                    binding_label(&keybinds.focus_pane_up),
                    Cow::Owned(rust_i18n::t!("keybind.focus_pane_up").to_string()),
                ),
                entry(
                    binding_label(&keybinds.focus_pane_right),
                    Cow::Owned(rust_i18n::t!("keybind.focus_pane_right").to_string()),
                ),
                entry(
                    binding_label(&keybinds.cycle_pane_next),
                    Cow::Owned(rust_i18n::t!("keybind.cycle_pane_next").to_string()),
                ),
                entry(
                    binding_label(&keybinds.cycle_pane_previous),
                    Cow::Owned(rust_i18n::t!("keybind.cycle_pane_previous").to_string()),
                ),
                entry(
                    binding_label(&keybinds.last_pane),
                    Cow::Owned(rust_i18n::t!("keybind.last_pane").to_string()),
                ),
            ],
        ),
    ];

    if !keybinds.custom_commands.is_empty() {
        groups.push((
            Cow::Owned(rust_i18n::t!("keybind.group_custom").to_string()),
            keybinds
                .custom_commands
                .iter()
                .map(|binding| {
                    (
                        binding.label.clone(),
                        binding
                            .description
                            .clone()
                            .map(Cow::Owned)
                            .unwrap_or(Cow::Owned(
                                rust_i18n::t!("keybind.custom_command").to_string(),
                            )),
                    )
                })
                .collect(),
        ));
    }
    groups
}

pub(crate) fn filter_keybind_help_groups(
    groups: Vec<KeybindHelpGroup>,
    query: &str,
) -> Vec<KeybindHelpGroup> {
    if query.is_empty() {
        return groups;
    }
    let query = query.to_lowercase();
    groups
        .into_iter()
        .filter_map(|(group, entries)| {
            let entries = entries
                .into_iter()
                .filter(|(key, label)| {
                    key.to_lowercase().contains(&query) || label.to_lowercase().contains(&query)
                })
                .collect::<Vec<_>>();
            (!entries.is_empty()).then_some((group, entries))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups() -> Vec<KeybindHelpGroup> {
        vec![
            (
                Cow::Borrowed("workspaces / tabs"),
                vec![
                    entry("w", Cow::Borrowed("workspace navigation")),
                    entry("c", Cow::Borrowed("new tab")),
                ],
            ),
            (
                Cow::Borrowed("panes"),
                vec![
                    entry("v", Cow::Borrowed("split vertical")),
                    entry("x", Cow::Borrowed("close pane")),
                ],
            ),
        ]
    }

    #[test]
    fn filter_matches_labels_and_shortcuts_case_insensitively() {
        let filtered = filter_keybind_help_groups(groups(), "WoRk");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1[0].1, "workspace navigation");

        let filtered = filter_keybind_help_groups(groups(), "x");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1[0].1, "close pane");
        assert!(filter_keybind_help_groups(groups(), "panes").is_empty());
    }
}
