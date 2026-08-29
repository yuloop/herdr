use std::borrow::Cow;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use rust_i18n::t;

use super::release_notes::release_notes_close_button_rect;
use super::scrollbar::{release_notes_scrollbar_rect, render_scrollbar};
use super::widgets::{
    modal_stack_areas, panel_contrast_fg, render_action_button, render_modal_header,
    render_modal_shell,
};
use crate::app::AppState;

pub(super) type HelpEntry = (String, Cow<'static, str>);
pub(super) type HelpGroup = (String, Vec<HelpEntry>);

fn help_entry(key: impl Into<String>, label: impl Into<Cow<'static, str>>) -> HelpEntry {
    (key.into(), label.into())
}

fn keybind_label(bindings: &crate::config::ActionKeybinds) -> String {
    bindings
        .label()
        .unwrap_or_else(|| t!("common.unset").to_string())
}

fn indexed_label(bindings: &[crate::config::IndexedKeybind]) -> String {
    if bindings.is_empty() {
        return t!("common.unset").to_string();
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

fn indexed_range_prefix(bindings: &[crate::config::IndexedKeybind]) -> Option<&str> {
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

pub(super) fn keybind_help_groups(app: &AppState) -> Vec<HelpGroup> {
    let kb = &app.keybinds;
    let mut groups = Vec::new();

    groups.push((
        t!("keybind.group_global").to_string(),
        vec![
            help_entry(
                crate::config::format_key_combo((app.prefix_code, app.prefix_mods)),
                t!("keybind.prefix_mode").to_string(),
            ),
            help_entry(keybind_label(&kb.help), t!("keybind.title").to_string()),
            help_entry(
                keybind_label(&kb.settings),
                t!("state.settings").to_string(),
            ),
            help_entry(keybind_label(&kb.detach), t!("state.detach").to_string()),
            help_entry(
                keybind_label(&kb.reload_config),
                t!("keybind.reload_config").to_string(),
            ),
            help_entry(
                keybind_label(&kb.open_notification_target),
                t!("keybind.open_notification_target").to_string(),
            ),
        ],
    ));

    groups.push((
        t!("keybind.group_navigation").to_string(),
        vec![
            help_entry("esc", t!("keybind.back").to_string()),
            help_entry(
                format!(
                    "{} / {}",
                    keybind_label(&kb.navigate.workspace_up),
                    keybind_label(&kb.navigate.workspace_down)
                ),
                t!("keybind.workspace_list").to_string(),
            ),
            help_entry(
                format!(
                    "{} / {} / {} / {} / left / right",
                    keybind_label(&kb.navigate.pane_left),
                    keybind_label(&kb.navigate.pane_down),
                    keybind_label(&kb.navigate.pane_up),
                    keybind_label(&kb.navigate.pane_right)
                ),
                t!("keybind.move_focus").to_string(),
            ),
            help_entry("tab / shift+tab", t!("keybind.cycle_pane").to_string()),
            help_entry("enter", t!("keybind.open_workspace").to_string()),
            help_entry("1..9", t!("keybind.switch_workspace").to_string()),
        ],
    ));

    let workspace_tab = vec![
        help_entry(
            keybind_label(&kb.workspace_picker),
            t!("keybind.workspace_navigation").to_string(),
        ),
        help_entry(
            keybind_label(&kb.goto),
            t!("keybind.session_navigator").to_string(),
        ),
        help_entry(
            keybind_label(&kb.new_workspace),
            t!("keybind.new_workspace").to_string(),
        ),
        help_entry(
            keybind_label(&kb.new_worktree),
            t!("keybind.new_worktree").to_string(),
        ),
        help_entry(
            keybind_label(&kb.open_worktree),
            t!("keybind.open_worktree").to_string(),
        ),
        help_entry(
            keybind_label(&kb.remove_worktree),
            t!("keybind.delete_worktree_checkout").to_string(),
        ),
        help_entry(
            keybind_label(&kb.rename_workspace),
            t!("keybind.rename_workspace").to_string(),
        ),
        help_entry(
            keybind_label(&kb.close_workspace),
            t!("keybind.close_workspace").to_string(),
        ),
        help_entry(
            keybind_label(&kb.previous_workspace),
            t!("keybind.previous_workspace").to_string(),
        ),
        help_entry(
            keybind_label(&kb.next_workspace),
            t!("keybind.next_workspace").to_string(),
        ),
        help_entry(
            indexed_label(&kb.switch_workspace),
            t!("keybind.switch_workspace_n").to_string(),
        ),
        help_entry(
            keybind_label(&kb.previous_agent),
            t!("keybind.previous_agent").to_string(),
        ),
        help_entry(
            keybind_label(&kb.next_agent),
            t!("keybind.next_agent").to_string(),
        ),
        help_entry(
            indexed_label(&kb.focus_agent),
            t!("keybind.focus_agent_n").to_string(),
        ),
        help_entry(
            keybind_label(&kb.new_tab),
            t!("keybind.new_tab").to_string(),
        ),
        help_entry(
            keybind_label(&kb.rename_tab),
            t!("keybind.rename_tab").to_string(),
        ),
        help_entry(
            keybind_label(&kb.previous_tab),
            t!("keybind.previous_tab").to_string(),
        ),
        help_entry(
            keybind_label(&kb.next_tab),
            t!("keybind.next_tab").to_string(),
        ),
        help_entry(
            indexed_label(&kb.switch_tab),
            t!("keybind.switch_tab_n").to_string(),
        ),
        help_entry(
            keybind_label(&kb.close_tab),
            t!("keybind.close_tab").to_string(),
        ),
    ];
    groups.push((t!("keybind.group_workspaces").to_string(), workspace_tab));

    let panes = vec![
        help_entry(
            keybind_label(&kb.split_vertical),
            t!("keybind.split_vertical").to_string(),
        ),
        help_entry(
            keybind_label(&kb.split_horizontal),
            t!("keybind.split_horizontal").to_string(),
        ),
        help_entry(
            keybind_label(&kb.close_pane),
            t!("keybind.close_pane").to_string(),
        ),
        help_entry(
            keybind_label(&kb.rename_pane),
            t!("keybind.rename_pane").to_string(),
        ),
        help_entry(
            keybind_label(&kb.edit_scrollback),
            t!("keybind.edit_scrollback").to_string(),
        ),
        help_entry(
            keybind_label(&kb.copy_mode),
            t!("keybind.copy_mode").to_string(),
        ),
        help_entry(keybind_label(&kb.zoom), t!("keybind.zoom_pane").to_string()),
        help_entry(
            keybind_label(&kb.resize_mode),
            t!("keybind.resize_mode").to_string(),
        ),
        help_entry(
            keybind_label(&kb.toggle_sidebar),
            t!("keybind.toggle_sidebar").to_string(),
        ),
        help_entry(
            keybind_label(&kb.focus_pane_left),
            t!("keybind.focus_pane_left").to_string(),
        ),
        help_entry(
            keybind_label(&kb.focus_pane_down),
            t!("keybind.focus_pane_down").to_string(),
        ),
        help_entry(
            keybind_label(&kb.focus_pane_up),
            t!("keybind.focus_pane_up").to_string(),
        ),
        help_entry(
            keybind_label(&kb.focus_pane_right),
            t!("keybind.focus_pane_right").to_string(),
        ),
        help_entry(
            keybind_label(&kb.cycle_pane_next),
            t!("keybind.cycle_pane_next").to_string(),
        ),
        help_entry(
            keybind_label(&kb.cycle_pane_previous),
            t!("keybind.cycle_pane_previous").to_string(),
        ),
        help_entry(
            keybind_label(&kb.last_pane),
            t!("keybind.last_pane").to_string(),
        ),
    ];
    groups.push((t!("keybind.group_panes").to_string(), panes));

    if !kb.custom_commands.is_empty() {
        groups.push((
            t!("keybind.group_custom").to_string(),
            kb.custom_commands
                .iter()
                .map(|binding| {
                    (
                        binding.label.clone(),
                        binding
                            .description
                            .clone()
                            .map(Cow::Owned)
                            .unwrap_or_else(|| t!("keybind.custom_command")),
                    )
                })
                .collect(),
        ));
    }

    groups
}

fn filter_keybind_help_groups(groups: Vec<HelpGroup>, query: &str) -> Vec<HelpGroup> {
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

pub(crate) fn keybind_help_lines(app: &AppState) -> Vec<(usize, Line<'static>)> {
    let heading_style = Style::default()
        .fg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default()
        .fg(app.palette.mauve)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(app.palette.text);

    let groups = filter_keybind_help_groups(keybind_help_groups(app), &app.keybind_help.query);
    let key_width = groups
        .iter()
        .flat_map(|(_, entries)| entries.iter().map(|(key, _)| key.chars().count()))
        .max()
        .unwrap_or(8);

    let mut lines = Vec::new();

    if groups.is_empty() {
        let message = " no matching keybinds";
        return vec![(
            message.chars().count(),
            Line::from(Span::styled(
                message,
                Style::default().fg(app.palette.overlay1),
            )),
        )];
    }

    for (group, entries) in groups {
        lines.push((
            group.len() + 1,
            Line::from(vec![Span::styled(format!(" {group}"), heading_style)]),
        ));
        for (key, label) in entries {
            let padded_key = format!(" {:<width$} ", key, width = key_width);
            let width = padded_key.chars().count() + label.chars().count();
            lines.push((
                width,
                Line::from(vec![
                    Span::styled(padded_key, key_style),
                    Span::styled(label.into_owned(), label_style),
                ]),
            ));
        }
        lines.push((0, Line::raw("")));
    }

    lines
}

pub(super) fn render_keybind_help_overlay(app: &AppState, frame: &mut Frame) {
    super::dim_background(frame, frame.area());

    let Some(inner) = render_modal_shell(frame, frame.area(), 76, 22, &app.palette) else {
        return;
    };
    if inner.height < 6 || inner.width < 20 {
        return;
    }

    let stack = modal_stack_areas(inner, 2, 1, 0, 1);
    let header_rows =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas::<2>(stack.header);

    render_modal_header(frame, header_rows[0], &t!("keybind.title"), &app.palette);
    let close_label = if app.keybind_help.search_focused {
        t!("keybind.back").to_string()
    } else {
        t!("common.close").to_string()
    };
    render_action_button(
        frame,
        release_notes_close_button_rect(header_rows[0]),
        Some("esc"),
        &close_label,
        Style::default()
            .fg(panel_contrast_fg(&app.palette))
            .bg(app.palette.accent)
            .add_modifier(Modifier::BOLD),
    );
    let search_line = if app.keybind_help.search_focused {
        Line::from(vec![
            Span::styled(
                " / ",
                Style::default()
                    .fg(app.palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                app.keybind_help.query.as_str(),
                Style::default()
                    .fg(app.palette.text)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(Span::styled(
            t!("keybind.filter_hint").to_string(),
            Style::default().fg(app.palette.overlay0),
        ))
    };
    frame.render_widget(Paragraph::new(search_line), header_rows[1]);

    let body_area = stack.content;
    let metrics = crate::pane::ScrollMetrics {
        offset_from_bottom: app
            .keybind_help_max_scroll()
            .saturating_sub(app.keybind_help.scroll) as usize,
        max_offset_from_bottom: app.keybind_help_max_scroll() as usize,
        viewport_rows: body_area.height.max(1) as usize,
    };
    let track = release_notes_scrollbar_rect(body_area, metrics);
    let text_area = track
        .map(|_| {
            Rect::new(
                body_area.x,
                body_area.y,
                body_area.width.saturating_sub(1),
                body_area.height,
            )
        })
        .unwrap_or(body_area);

    let body = Paragraph::new(
        keybind_help_lines(app)
            .into_iter()
            .map(|(_, line)| line)
            .collect::<Vec<_>>(),
    )
    .wrap(Wrap { trim: false })
    .scroll((app.keybind_help.scroll, 0));
    frame.render_widget(body, text_area);
    if let Some(track) = track {
        render_scrollbar(
            frame,
            metrics,
            track,
            app.palette.overlay0,
            app.palette.overlay1,
            "▐",
        );
    }

    let footer = if app.keybind_help.search_focused {
        Line::from(vec![
            Span::styled(
                t!("keybind.filter_label").to_string(),
                Style::default().fg(app.palette.overlay0),
            ),
            Span::styled(
                t!("keybind.type_backspace").to_string(),
                Style::default().fg(app.palette.text),
            ),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled(
                format!("{} ", t!("common.clear")),
                Style::default().fg(app.palette.overlay0),
            ),
            Span::styled("ctrl+u", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled(
                format!("{} ", t!("common.scroll").trim()),
                Style::default().fg(app.palette.overlay0),
            ),
            Span::styled("↑↓/pgup/pgdn", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled(
                format!("{} ", t!("keybind.back")),
                Style::default().fg(app.palette.overlay0),
            ),
            Span::styled("esc", Style::default().fg(app.palette.text)),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                t!("keybind.search_label").to_string(),
                Style::default().fg(app.palette.overlay0),
            ),
            Span::styled("/", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled(
                format!("{} ", t!("common.scroll").trim()),
                Style::default().fg(app.palette.overlay0),
            ),
            Span::styled("j/k/↑↓/pgup/pgdn", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled(
                format!("{} ", t!("common.close")),
                Style::default().fg(app.palette.overlay0),
            ),
            Span::styled("esc/enter", Style::default().fg(app.palette.text)),
        ])
    };
    frame.render_widget(Paragraph::new(footer), stack.footer.unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups() -> Vec<HelpGroup> {
        vec![
            (
                "workspaces / tabs".to_string(),
                vec![
                    help_entry("w", "workspace navigation"),
                    help_entry("c", "new tab"),
                ],
            ),
            (
                "panes".to_string(),
                vec![
                    help_entry("v", "split vertical"),
                    help_entry("x", "close pane"),
                ],
            ),
        ]
    }

    #[test]
    fn keybind_help_filter_matches_labels_case_insensitively() {
        let filtered = filter_keybind_help_groups(groups(), "WoRk");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "workspaces / tabs");
        assert_eq!(filtered[0].1.len(), 1);
        assert_eq!(filtered[0].1[0].1, "workspace navigation");
    }

    #[test]
    fn keybind_help_filter_matches_shortcuts_without_matching_group_headings() {
        let filtered = filter_keybind_help_groups(groups(), "x");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "panes");
        assert_eq!(filtered[0].1.len(), 1);
        assert_eq!(filtered[0].1[0].1, "close pane");

        assert!(filter_keybind_help_groups(groups(), "panes").is_empty());
    }
}
