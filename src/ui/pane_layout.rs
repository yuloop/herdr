use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use rust_i18n::t;

use crate::{
    app::state::{AppState, PaneLayoutInteraction, PaneTransferDestination, PreviewPaneRole},
    layout::LayoutPreset,
    terminal::TerminalRuntimeRegistry,
};

use super::widgets::{panel_contrast_fg, render_panel_shell};

pub(super) fn render_pane_layout_overlay(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    area: Rect,
) {
    let Some(interaction) = app.pane_layout.as_ref() else {
        return;
    };
    match &interaction.interaction {
        PaneLayoutInteraction::Transfer(_) => {
            render_transfer_preview(app, frame);
            render_transfer_targets(app, terminal_runtimes, frame, area);
        }
        PaneLayoutInteraction::Reposition { .. } | PaneLayoutInteraction::Preset { .. } => {
            let Some(preview) = app.pane_layout_preview() else {
                return;
            };
            let target_pane_id = match &interaction.interaction {
                PaneLayoutInteraction::Reposition { target_pane_id, .. } => Some(*target_pane_id),
                PaneLayoutInteraction::Preset { .. } | PaneLayoutInteraction::Transfer(_) => None,
            };
            for pane in preview.panes(area) {
                if pane.rect.width < 2 || pane.rect.height < 2 {
                    continue;
                }
                let (color, title) = if pane.id == interaction.source_pane_id {
                    (app.palette.yellow, t!("state.layout_source").to_string())
                } else if Some(pane.id) == target_pane_id {
                    (app.palette.blue, t!("state.layout_target").to_string())
                } else {
                    (app.palette.overlay1, String::new())
                };
                render_preview_block(frame, pane.rect, color, title);
            }
            if matches!(
                &interaction.interaction,
                PaneLayoutInteraction::Preset { .. }
            ) {
                render_preset_picker(app, frame, area);
            }
        }
    }
    render_instruction_bar(app, terminal_runtimes, frame, area);
}

fn render_transfer_preview(app: &AppState, frame: &mut Frame) {
    let Some(preview) = app.pane_transfer_preview() else {
        return;
    };
    for pane in preview {
        if pane.rect.width < 2 || pane.rect.height < 2 {
            continue;
        }
        let (color, title) = match pane.role {
            PreviewPaneRole::Source => (app.palette.yellow, t!("state.layout_source").to_string()),
            PreviewPaneRole::Target => (app.palette.blue, t!("state.layout_target").to_string()),
            PreviewPaneRole::Existing => (app.palette.overlay1, String::new()),
        };
        render_preview_block(frame, pane.rect, color, title);
    }
}

fn render_preview_block(
    frame: &mut Frame,
    rect: Rect,
    color: ratatui::style::Color,
    title: String,
) {
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color));
    if !title.is_empty() {
        block = block.title(Span::styled(
            format!(" {title} "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }
    frame.render_widget(block, rect);
}

fn render_transfer_targets(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    area: Rect,
) {
    let Some(PaneLayoutInteraction::Transfer(transfer)) =
        app.pane_layout.as_ref().map(|layout| &layout.interaction)
    else {
        return;
    };
    let candidates = app.pane_transfer_candidates();
    let picker = pane_transfer_target_picker_rect(area, candidates.len());
    if picker == Rect::default() {
        return;
    }
    let selected_idx = transfer
        .selected
        .as_ref()
        .and_then(|selected| {
            candidates
                .iter()
                .position(|candidate| &candidate.destination == selected)
        })
        .unwrap_or(0);
    frame.render_widget(Clear, picker);
    let Some(inner) = render_panel_shell(frame, picker, app.palette.accent, app.palette.panel_bg)
    else {
        return;
    };
    let visible = inner.height as usize;
    let start = pane_transfer_visible_start(candidates.len(), selected_idx, visible);
    for (row, candidate) in candidates.iter().skip(start).take(visible).enumerate() {
        let idx = start + row;
        let selected = idx == selected_idx;
        let style = if selected {
            Style::default()
                .bg(app.palette.accent)
                .fg(panel_contrast_fg(&app.palette))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.palette.text)
        };
        frame.render_widget(
            Paragraph::new(format!(
                " {}  {}",
                idx + 1,
                transfer_destination_label(app, terminal_runtimes, &candidate.destination)
            ))
            .style(style),
            Rect::new(inner.x, inner.y + row as u16, inner.width, 1),
        );
    }
}

pub(crate) fn pane_transfer_destination_at(
    app: &AppState,
    area: Rect,
    col: u16,
    row: u16,
) -> Option<PaneTransferDestination> {
    let PaneLayoutInteraction::Transfer(transfer) = &app.pane_layout.as_ref()?.interaction else {
        return None;
    };
    let candidates = app.pane_transfer_candidates();
    let picker = pane_transfer_target_picker_rect(area, candidates.len());
    if picker == Rect::default() {
        return None;
    }
    let inner = Block::default().borders(Borders::ALL).inner(picker);
    if col < inner.x
        || col >= inner.x.saturating_add(inner.width)
        || row < inner.y
        || row >= inner.y.saturating_add(inner.height)
    {
        return None;
    }
    let selected_idx = transfer
        .selected
        .as_ref()
        .and_then(|selected| {
            candidates
                .iter()
                .position(|candidate| &candidate.destination == selected)
        })
        .unwrap_or(0);
    let start = pane_transfer_visible_start(candidates.len(), selected_idx, inner.height as usize);
    candidates
        .get(start + row.saturating_sub(inner.y) as usize)
        .map(|candidate| candidate.destination.clone())
}

fn pane_transfer_target_picker_rect(area: Rect, candidate_count: usize) -> Rect {
    if candidate_count == 0 || area.width < 24 || area.height < 5 {
        return Rect::default();
    }
    let desired_height = (candidate_count.min(usize::from(u16::MAX)) as u16).saturating_add(2);
    let height = area.height.saturating_sub(2).min(desired_height);
    let width = area.width.min(44);
    Rect::new(
        area.x.saturating_add(area.width.saturating_sub(width)),
        area.y,
        width,
        height,
    )
}

fn pane_transfer_visible_start(
    candidate_count: usize,
    selected_idx: usize,
    visible: usize,
) -> usize {
    selected_idx
        .saturating_sub(visible.saturating_sub(1))
        .min(candidate_count.saturating_sub(visible))
}

fn transfer_destination_label(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    destination: &PaneTransferDestination,
) -> String {
    match destination {
        PaneTransferDestination::PaneEdge {
            workspace_id,
            tab_id,
            pane_id,
            placement,
        } => {
            let direction = match placement {
                crate::layout::PanePlacement::Left => "←",
                crate::layout::PanePlacement::Right => "→",
                crate::layout::PanePlacement::Up => "↑",
                crate::layout::PanePlacement::Down => "↓",
            };
            let Some((ws_idx, tab_idx, resolved_pane_id)) =
                app.resolve_pane_transfer_identity(workspace_id, tab_id, pane_id)
            else {
                return format!("{pane_id} · {direction}");
            };
            let workspace = &app.workspaces[ws_idx];
            let workspace_label = workspace.display_name_from(&app.terminals, terminal_runtimes);
            let tab_label = workspace
                .tab_display_name(tab_idx)
                .unwrap_or_else(|| tab_id.clone());
            let pane_label = workspace.tabs[tab_idx]
                .terminal_id(resolved_pane_id)
                .and_then(|terminal_id| app.terminals.get(terminal_id))
                .and_then(|terminal| terminal.border_label(app.show_agent_labels_on_pane_borders))
                .filter(|label| !label.trim().is_empty());
            match pane_label {
                Some(pane_label) => {
                    format!(
                        "{pane_id} · {pane_label} · {workspace_label} / {tab_label} · {direction}"
                    )
                }
                None => format!("{pane_id} · {workspace_label} / {tab_label} · {direction}"),
            }
        }
        PaneTransferDestination::NewTab { workspace_id } => {
            let is_source_workspace = app
                .pane_layout
                .as_ref()
                .and_then(crate::app::state::PaneLayoutInteractionState::transfer_source)
                .is_some_and(|source| source.workspace_id == *workspace_id);
            let action_label = if is_source_workspace {
                t!("state.detach")
            } else {
                t!("state.pane_transfer_move")
            };
            let new_tab_label = format!("{} → {}", action_label, t!("state.pane_transfer_new_tab"));
            let workspace_label = app
                .workspaces
                .iter()
                .find(|workspace| workspace.id == *workspace_id)
                .map(|workspace| workspace.display_name_from(&app.terminals, terminal_runtimes));
            match workspace_label {
                Some(workspace_label) => {
                    format!("{workspace_id} · {workspace_label} · {new_tab_label}")
                }
                None => format!("{workspace_id} · {new_tab_label}"),
            }
        }
        PaneTransferDestination::NewWorkspace => format!(
            "{} → {}",
            t!("state.detach"),
            t!("state.pane_transfer_new_workspace")
        ),
    }
}

fn render_preset_picker(app: &AppState, frame: &mut Frame, area: Rect) {
    let Some(PaneLayoutInteraction::Preset { preset }) =
        app.pane_layout.as_ref().map(|layout| &layout.interaction)
    else {
        return;
    };
    let preset = *preset;
    let picker = pane_layout_preset_picker_rect(area);
    if picker.width == 0 || picker.height == 0 {
        return;
    }
    frame.render_widget(Clear, picker);
    let Some(inner) = render_panel_shell(frame, picker, app.palette.accent, app.palette.panel_bg)
    else {
        return;
    };
    for (idx, candidate) in LayoutPreset::ALL.iter().copied().enumerate() {
        if idx as u16 >= inner.height {
            break;
        }
        let selected = candidate == preset;
        let style = if selected {
            Style::default()
                .bg(app.palette.accent)
                .fg(panel_contrast_fg(&app.palette))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.palette.text)
        };
        frame.render_widget(
            Paragraph::new(format!(" {}  {}", idx + 1, preset_label(candidate))).style(style),
            Rect::new(inner.x, inner.y + idx as u16, inner.width, 1),
        );
    }
}

fn render_instruction_bar(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    area: Rect,
) {
    if area.height == 0 {
        return;
    }
    let Some(interaction) = app.pane_layout.as_ref() else {
        return;
    };
    let (title, hint) = match &interaction.interaction {
        PaneLayoutInteraction::Reposition { .. } => (
            t!("state.layout_move_title").to_string(),
            t!("state.layout_move_hint").to_string(),
        ),
        PaneLayoutInteraction::Preset { .. } => (
            t!("state.layout_templates_title").to_string(),
            t!("state.layout_templates_hint").to_string(),
        ),
        PaneLayoutInteraction::Transfer(transfer) => {
            let target = transfer
                .selected
                .as_ref()
                .map(|destination| transfer_destination_label(app, terminal_runtimes, destination))
                .unwrap_or_else(|| "—".into());
            (
                t!("state.pane_transfer_title").to_string(),
                format!("{target} · {}", t!("state.layout_move_hint")),
            )
        }
    };
    let bar = Rect::new(
        area.x,
        area.y.saturating_add(area.height.saturating_sub(1)),
        area.width,
        1,
    );
    frame.render_widget(Clear, bar);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {title} "),
                Style::default()
                    .bg(app.palette.mauve)
                    .fg(panel_contrast_fg(&app.palette))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(hint, Style::default().fg(app.palette.subtext0)),
        ]))
        .style(Style::default().bg(app.palette.panel_bg))
        .alignment(Alignment::Left),
        bar,
    );
}

pub(crate) fn pane_layout_preset_at(area: Rect, col: u16, row: u16) -> Option<LayoutPreset> {
    let picker = pane_layout_preset_picker_rect(area);
    if picker.width < 3
        || picker.height < 3
        || col <= picker.x
        || col >= picker.x.saturating_add(picker.width).saturating_sub(1)
        || row <= picker.y
        || row >= picker.y.saturating_add(picker.height).saturating_sub(1)
    {
        return None;
    }
    LayoutPreset::ALL
        .get(row.saturating_sub(picker.y).saturating_sub(1) as usize)
        .copied()
}

fn pane_layout_preset_picker_rect(area: Rect) -> Rect {
    let width = area.width.min(30);
    let height = area.height.min(LayoutPreset::ALL.len() as u16 + 2);
    if width < 4 || height < 3 {
        return Rect::default();
    }
    Rect::new(
        area.x.saturating_add(area.width.saturating_sub(width)),
        area.y,
        width,
        height,
    )
}

fn preset_label(preset: LayoutPreset) -> String {
    match preset {
        LayoutPreset::Columns => t!("state.layout_preset_columns"),
        LayoutPreset::Rows => t!("state.layout_preset_rows"),
        LayoutPreset::Grid => t!("state.layout_preset_grid"),
        LayoutPreset::MainLeft => t!("state.layout_preset_main_left"),
        LayoutPreset::MainTop => t!("state.layout_preset_main_top"),
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Direction;

    #[test]
    fn preset_picker_maps_each_content_row_and_rejects_its_border() {
        let area = Rect::new(5, 3, 80, 24);
        let picker = pane_layout_preset_picker_rect(area);
        assert_eq!(picker.width, 30);
        assert_eq!(picker.height, LayoutPreset::ALL.len() as u16 + 2);

        for (idx, preset) in LayoutPreset::ALL.iter().copied().enumerate() {
            assert_eq!(
                pane_layout_preset_at(area, picker.x + 2, picker.y + 1 + idx as u16),
                Some(preset)
            );
        }
        assert_eq!(pane_layout_preset_at(area, picker.x, picker.y + 1), None);
        assert_eq!(
            pane_layout_preset_at(
                area,
                picker.x + picker.width.saturating_sub(1),
                picker.y + 1
            ),
            None
        );
    }

    #[test]
    fn transfer_labels_include_user_context_and_stable_pane_identity() {
        let mut app = AppState::test_new();
        let mut workspace = crate::workspace::Workspace::test_new("项目甲");
        workspace.tabs[0].set_custom_name("主线".into());
        let first_pane = workspace.tabs[0].root_pane;
        let second_pane = workspace.test_split(Direction::Horizontal);
        let workspace_id = workspace.id.clone();
        let tab_id =
            crate::workspace::public_tab_id_for_number(&workspace_id, workspace.tabs[0].number);
        let first_public_id = crate::workspace::public_pane_id_for_number(
            &workspace_id,
            workspace
                .public_pane_number(first_pane)
                .expect("first public number"),
        );
        let second_public_id = crate::workspace::public_pane_id_for_number(
            &workspace_id,
            workspace
                .public_pane_number(second_pane)
                .expect("second public number"),
        );
        let target_workspace = crate::workspace::Workspace::test_new("项目乙");
        let target_workspace_id = target_workspace.id.clone();
        app.workspaces = vec![workspace, target_workspace];
        app.ensure_test_terminals();
        for pane_id in [first_pane, second_pane] {
            let terminal_id = app.workspaces[0].tabs[0]
                .terminal_id(pane_id)
                .expect("terminal id");
            app.terminals
                .get_mut(terminal_id)
                .expect("terminal state")
                .set_manual_label("服务".into());
        }
        let source = app
            .pane_transfer_source(0, 0, first_pane)
            .expect("transfer source");
        app.pane_layout = Some(crate::app::state::PaneLayoutInteractionState {
            ws_idx: 0,
            tab_idx: 0,
            source_pane_id: first_pane,
            interaction: PaneLayoutInteraction::Transfer(crate::app::state::PaneTransferState {
                source,
                origin: crate::app::state::PaneTransferOrigin::ContextMenu,
                selected: None,
            }),
        });

        let runtimes = TerminalRuntimeRegistry::new();
        let first = transfer_destination_label(
            &app,
            &runtimes,
            &PaneTransferDestination::PaneEdge {
                workspace_id: workspace_id.clone(),
                tab_id: tab_id.clone(),
                pane_id: first_public_id.clone(),
                placement: crate::layout::PanePlacement::Left,
            },
        );
        let second = transfer_destination_label(
            &app,
            &runtimes,
            &PaneTransferDestination::PaneEdge {
                workspace_id: workspace_id.clone(),
                tab_id,
                pane_id: second_public_id.clone(),
                placement: crate::layout::PanePlacement::Left,
            },
        );
        let new_tab = transfer_destination_label(
            &app,
            &runtimes,
            &PaneTransferDestination::NewTab {
                workspace_id: workspace_id.clone(),
            },
        );
        let moved_new_tab = transfer_destination_label(
            &app,
            &runtimes,
            &PaneTransferDestination::NewTab {
                workspace_id: target_workspace_id,
            },
        );

        for label in [&first, &second, &new_tab] {
            assert!(
                label.contains("项目甲"),
                "missing workspace context: {label}"
            );
        }
        for label in [&first, &second] {
            assert!(label.contains("主线"), "missing tab context: {label}");
            assert!(label.contains("服务"), "missing pane title: {label}");
        }
        assert!(first.contains(&first_public_id));
        assert!(second.contains(&second_public_id));
        assert!(new_tab.contains(t!("state.detach").as_ref()));
        assert!(moved_new_tab.contains(t!("state.pane_transfer_move").as_ref()));
        assert!(
            !moved_new_tab.contains(t!("state.detach").as_ref()),
            "moving into another workspace is not a same-workspace detach"
        );
        assert_ne!(
            first, second,
            "stable pane identity must disambiguate titles"
        );
    }

    #[test]
    fn transfer_picker_shows_all_session_rows_when_height_allows() {
        let mut app = AppState::test_new();
        let source_workspace = crate::workspace::Workspace::test_new("source");
        let source_pane_id = source_workspace.tabs[0].root_pane;
        app.workspaces = std::iter::once(source_workspace)
            .chain(
                (0..6).map(|idx| crate::workspace::Workspace::test_new(&format!("target-{idx}"))),
            )
            .collect();
        let source = app
            .pane_transfer_source(0, 0, source_pane_id)
            .expect("transfer source");
        app.pane_layout = Some(crate::app::state::PaneLayoutInteractionState {
            ws_idx: 0,
            tab_idx: 0,
            source_pane_id,
            interaction: PaneLayoutInteraction::Transfer(crate::app::state::PaneTransferState {
                source: source.clone(),
                origin: crate::app::state::PaneTransferOrigin::ContextMenu,
                selected: None,
            }),
        });
        let area = Rect::new(0, 0, 80, 24);
        let candidates = app.pane_transfer_candidates();
        let picker = pane_transfer_target_picker_rect(area, candidates.len());

        assert_eq!(
            candidates.len(),
            13,
            "one new-workspace row, six session rows, and six meaningful new-tab rows"
        );
        assert_eq!(
            picker.height,
            candidates.len() as u16 + 2,
            "the picker should not truncate to ten content rows when space is available"
        );
        assert!(matches!(
            pane_transfer_destination_at(&app, area, picker.x + 1, picker.y + 1),
            Some(PaneTransferDestination::NewWorkspace)
        ));
        assert!(matches!(
            pane_transfer_destination_at(&app, area, picker.x + 1, picker.y + 2),
            Some(PaneTransferDestination::PaneEdge { .. })
        ));
        assert!(matches!(
            pane_transfer_destination_at(
                &app,
                area,
                picker.x + 1,
                picker.y + candidates.len() as u16
            ),
            Some(PaneTransferDestination::NewTab { .. })
        ));
    }
}
