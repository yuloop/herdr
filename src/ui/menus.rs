use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};
use rust_i18n::t;

use super::widgets::{panel_contrast_fg, render_panel_shell};
use crate::app::AppState;

fn prefix_rhs_label(bindings: &crate::config::ActionKeybinds) -> String {
    bindings
        .prefix_rhs_label()
        .unwrap_or_else(|| t!("common.unset").to_string())
}

fn keybind_label(bindings: &crate::config::ActionKeybinds) -> String {
    bindings
        .label()
        .unwrap_or_else(|| t!("common.unset").to_string())
}

fn render_bottom_bar(frame: &mut Frame, area: Rect, line: Line<'_>, bg: ratatui::style::Color) {
    frame.render_widget(Clear, area);
    let buf = frame.buffer_mut();
    for x in area.x..area.x + area.width {
        buf[(x, area.y)].set_style(Style::default().bg(bg));
    }
    frame.render_widget(Paragraph::new(line), area);
}

pub(super) fn render_prefix_overlay(app: &AppState, frame: &mut Frame, area: Rect) {
    let key = Style::default()
        .fg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(app.palette.overlay0);
    let mode_style = Style::default()
        .fg(panel_contrast_fg(&app.palette))
        .bg(app.palette.accent)
        .add_modifier(Modifier::BOLD);

    let workspace_picker = prefix_rhs_label(&app.keybinds.workspace_picker);
    let help = prefix_rhs_label(&app.keybinds.help);
    let prefix = crate::config::format_key_combo((app.prefix_code, app.prefix_mods));

    let line = Line::from(vec![
        Span::styled(t!("menu.prefix_label").to_string(), mode_style),
        Span::raw(" "),
        Span::styled("esc", key),
        Span::styled(t!("menu.cancel_hint").to_string(), dim),
        Span::styled(prefix, key),
        Span::styled(t!("menu.send_prefix").to_string(), dim),
        Span::styled(workspace_picker, key),
        Span::styled(t!("menu.workspace_nav_hint").to_string(), dim),
        Span::styled(help, key),
        Span::styled(t!("menu.keybinds_hint").to_string(), dim),
    ]);

    let overlay_y = area.y + area.height.saturating_sub(1);
    let overlay_area = Rect::new(area.x, overlay_y, area.width, 1);
    render_bottom_bar(frame, overlay_area, line, app.palette.panel_bg);
}

pub(super) fn render_copy_mode_overlay(app: &AppState, frame: &mut Frame, area: Rect) {
    let key = Style::default()
        .fg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(app.palette.overlay0);
    let mode_style = Style::default()
        .fg(panel_contrast_fg(&app.palette))
        .bg(app.palette.accent)
        .add_modifier(Modifier::BOLD);

    let Some(copy_mode) = app.copy_mode.as_ref() else {
        return;
    };
    let line = if let Some(prompt) = copy_mode.search.prompt.as_ref() {
        let marker = match prompt.direction {
            crate::app::state::CopyModeSearchDirection::Forward => "/",
            crate::app::state::CopyModeSearchDirection::Backward => "?",
        };
        Line::from(vec![
            Span::styled(t!("menu.copy_label").to_string(), mode_style),
            Span::raw(" "),
            Span::styled(marker, key),
            Span::styled(prompt.query.clone(), Style::default().fg(app.palette.text)),
            Span::styled("█", key),
            Span::styled(t!("menu.enter_search_cancel").to_string(), dim),
        ])
    } else {
        let select = if copy_mode.selection.is_some() {
            t!("menu.selecting").to_string()
        } else {
            t!("menu.select").to_string()
        };
        let match_status = copy_mode
            .search
            .current
            .map(|current| format!(" {}/{}", current + 1, copy_mode.search.matches.len()))
            .or_else(|| (!copy_mode.search.query.is_empty()).then(|| " 0/0".to_string()))
            .unwrap_or_default();
        let (exit_keys, exit_label) =
            if copy_mode.search.query.is_empty() && copy_mode.selection.is_none() {
                ("q/esc", t!("menu.exit_hint").to_string())
            } else {
                ("esc", t!("menu.clear_exit_hint").to_string())
            };
        Line::from(vec![
            Span::styled(t!("menu.copy_label").to_string(), mode_style),
            Span::raw(" "),
            Span::styled("h/j/k/l w/b/e { }", key),
            Span::styled(t!("menu.move_hint").to_string(), dim),
            Span::styled("/ ?", key),
            Span::styled(t!("menu.search_hint").to_string(), dim),
            Span::styled("n/N", key),
            Span::styled(format!(" {}{match_status}  ", t!("menu.repeat")), dim),
            Span::styled("v/space", key),
            Span::styled(format!(" {select}  "), dim),
            Span::styled("y/enter", key),
            Span::styled(t!("menu.copy_hint").to_string(), dim),
            Span::styled(exit_keys, key),
            Span::styled(exit_label, dim),
        ])
    };

    let overlay_y = area.y + area.height.saturating_sub(1);
    let overlay_area = Rect::new(area.x, overlay_y, area.width, 1);
    render_bottom_bar(frame, overlay_area, line, app.palette.panel_bg);
}

pub(super) fn render_navigate_overlay(app: &AppState, frame: &mut Frame, area: Rect) {
    let key = Style::default()
        .fg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(app.palette.overlay0);

    let mode_style = Style::default()
        .fg(panel_contrast_fg(&app.palette))
        .bg(app.palette.accent)
        .add_modifier(Modifier::BOLD);

    let kb = &app.keybinds;
    let new_tab = prefix_rhs_label(&kb.new_tab);
    let split_vertical = prefix_rhs_label(&kb.split_vertical);
    let split_horizontal = prefix_rhs_label(&kb.split_horizontal);
    let close_pane = prefix_rhs_label(&kb.close_pane);
    let zoom = prefix_rhs_label(&kb.zoom);
    let resize = prefix_rhs_label(&kb.resize_mode);
    let help = prefix_rhs_label(&kb.help);
    let settings = prefix_rhs_label(&kb.settings);
    let goto = prefix_rhs_label(&kb.goto);
    let detach = prefix_rhs_label(&kb.detach);
    let workspace_nav = format!(
        "{} / {}",
        keybind_label(&kb.navigate.workspace_up),
        keybind_label(&kb.navigate.workspace_down)
    );
    let line = Line::from(vec![
        Span::styled(t!("menu.navigate_label").to_string(), mode_style),
        Span::raw(" "),
        Span::styled("esc", key),
        Span::styled(t!("menu.back_hint").to_string(), dim),
        Span::styled(workspace_nav, key),
        Span::styled(t!("menu.ws_hint").to_string(), dim),
        Span::styled("⇥", key),
        Span::styled(t!("menu.pane_hint").to_string(), dim),
        Span::styled(goto, key),
        Span::styled(t!("menu.navigator_hint").to_string(), dim),
        Span::styled(new_tab, key),
        Span::styled(t!("menu.new_tab_hint").to_string(), dim),
        Span::styled(split_vertical, key),
        Span::styled(t!("menu.split_v_hint").to_string(), dim),
        Span::styled(split_horizontal, key),
        Span::styled(t!("menu.split_h_hint").to_string(), dim),
        Span::styled(close_pane, key),
        Span::styled(t!("menu.close_hint").to_string(), dim),
        Span::styled(zoom, key),
        Span::styled(t!("menu.zoom_hint").to_string(), dim),
        Span::styled(resize, key),
        Span::styled(t!("menu.resize_hint").to_string(), dim),
        Span::styled(help, key),
        Span::styled(t!("menu.keybinds_hint_nav").to_string(), dim),
        Span::styled(settings, key),
        Span::styled(t!("menu.settings_hint").to_string(), dim),
        Span::styled(detach, key),
        Span::styled(t!("menu.detach_hint").to_string(), dim),
    ]);

    let overlay_y = area.y + area.height.saturating_sub(1);
    let overlay_area = Rect::new(area.x, overlay_y, area.width, 1);
    render_bottom_bar(frame, overlay_area, line, app.palette.panel_bg);

    if app.update_available.is_some() {
        let status = Line::from(vec![Span::styled(
            t!("menu.update_ready").to_string(),
            Style::default()
                .fg(app.palette.accent)
                .add_modifier(Modifier::BOLD),
        )]);
        let width = 13u16.min(overlay_area.width);
        let status_area = Rect::new(
            overlay_area.x + overlay_area.width.saturating_sub(width),
            overlay_area.y,
            width,
            overlay_area.height,
        );
        frame.render_widget(Clear, status_area);
        frame.render_widget(
            Paragraph::new(status).alignment(Alignment::Right),
            status_area,
        );
    }
}

pub(super) fn render_global_launcher_menu(app: &AppState, frame: &mut Frame) {
    let rect = app.global_menu_rect();
    let Some(inner) = render_panel_shell(frame, rect, app.palette.accent, app.palette.panel_bg)
    else {
        return;
    };

    let items = app.global_menu_labels();
    for (idx, item) in items.iter().enumerate() {
        let y = inner.y + idx as u16;
        if y >= inner.y + inner.height {
            break;
        }
        let selected = idx == app.global_menu.highlighted;
        let rect = Rect::new(inner.x, y, inner.width, 1);

        let selected_style = Style::default()
            .fg(panel_contrast_fg(&app.palette))
            .bg(app.palette.accent)
            .add_modifier(Modifier::BOLD);
        let item_style = if selected {
            selected_style
        } else {
            Style::default().fg(app.palette.text)
        };
        let badge_style = if selected {
            selected_style
        } else {
            Style::default()
                .fg(app.palette.accent)
                .add_modifier(Modifier::BOLD)
        };

        let line = if app.global_menu_item_has_badge(item) {
            Line::from(vec![
                Span::styled(" ●", badge_style),
                Span::styled(
                    format!(" {} ", app.global_menu_display_label(item)),
                    item_style,
                ),
            ])
        } else {
            Line::from(Span::styled(
                format!(" {} ", app.global_menu_display_label(item)),
                item_style,
            ))
        };
        frame.render_widget(Paragraph::new(line).alignment(Alignment::Left), rect);
    }
}

pub(super) fn render_resize_overlay(app: &AppState, frame: &mut Frame, area: Rect) {
    let key = Style::default()
        .fg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(app.palette.overlay0);

    let mode_style = Style::default()
        .fg(panel_contrast_fg(&app.palette))
        .bg(app.palette.mauve)
        .add_modifier(Modifier::BOLD);

    let line = Line::from(vec![
        Span::styled(t!("menu.resize_label").to_string(), mode_style),
        Span::raw("  "),
        Span::styled("h/l", key),
        Span::styled(t!("menu.width_hint").to_string(), dim),
        Span::styled("j/k", key),
        Span::styled(t!("menu.height_hint").to_string(), dim),
        Span::styled("esc", key),
        Span::styled(t!("menu.done_hint").to_string(), dim),
    ]);

    let overlay_y = area.y + area.height.saturating_sub(1);
    let overlay_area = Rect::new(area.x, overlay_y, area.width, 1);
    render_bottom_bar(frame, overlay_area, line, app.palette.panel_bg);
}

pub(super) fn render_context_menu(app: &AppState, frame: &mut Frame) {
    let Some(menu) = &app.context_menu else {
        return;
    };

    let p = &app.palette;
    let Some(menu_rect) = app.context_menu_rect() else {
        return;
    };
    let Some(inner) = render_panel_shell(frame, menu_rect, p.accent, p.panel_bg) else {
        return;
    };

    let actions = menu.actions();
    let mut visual_row = 0u16;
    for (idx, action) in actions.iter().copied().enumerate() {
        if menu.has_separator_before(idx) {
            let separator = "─".repeat(inner.width as usize);
            frame.render_widget(
                Paragraph::new(separator).style(Style::default().fg(p.surface1)),
                Rect::new(inner.x, inner.y + visual_row, inner.width, 1),
            );
            visual_row = visual_row.saturating_add(1);
        }
        if visual_row >= inner.height {
            break;
        }
        let selected = idx == menu.list.highlighted;
        let style = if selected {
            Style::default()
                .bg(p.accent)
                .fg(panel_contrast_fg(p))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.text)
        };
        frame.render_widget(
            Paragraph::new(format!(" {}", action.display_label())).style(style),
            Rect::new(inner.x, inner.y + visual_row, inner.width, 1),
        );
        visual_row = visual_row.saturating_add(1);
    }
}
