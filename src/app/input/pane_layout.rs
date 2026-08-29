use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rust_i18n::t;

use crate::{
    api::schema::{
        LayoutRearrangeOperation, LayoutRearrangeParams, PaneDirection, PaneLayoutPreset,
        PaneMoveDestination as ApiPaneMoveDestination, PaneMoveParams, ResponseResult,
        SplitDirection, SuccessResponse,
    },
    app::{
        state::{
            AppState, Mode, PaneLayoutInteraction, PaneLayoutInteractionState,
            PaneTransferCandidate, PaneTransferDestination, PaneTransferOrigin, PaneTransferState,
        },
        App,
    },
    layout::{LayoutPreset, PaneId, PanePlacement},
};

use super::modal::leave_modal;

#[cfg(test)]
use crate::app::state::PreviewPaneRole;

pub(super) fn open_pane_reposition(
    state: &mut AppState,
    ws_idx: usize,
    tab_idx: usize,
    source_pane_id: PaneId,
) -> bool {
    let Some(tab) = state
        .workspaces
        .get(ws_idx)
        .and_then(|workspace| workspace.tabs.get(tab_idx))
    else {
        return false;
    };
    if tab.zoomed || tab.layout.pane_count() <= 1 {
        return false;
    }
    let Some(target_pane_id) = visual_pane_order(state, ws_idx, tab_idx)
        .into_iter()
        .find(|pane_id| *pane_id != source_pane_id)
    else {
        return false;
    };

    state.context_menu = None;
    state.pane_layout = Some(PaneLayoutInteractionState {
        ws_idx,
        tab_idx,
        source_pane_id,
        interaction: PaneLayoutInteraction::Reposition {
            target_pane_id,
            placement: PanePlacement::Right,
        },
    });
    state.mode = Mode::PaneLayout;
    true
}

pub(super) fn open_pane_layout_presets(
    state: &mut AppState,
    ws_idx: usize,
    tab_idx: usize,
    source_pane_id: PaneId,
) -> bool {
    let Some(tab) = state
        .workspaces
        .get(ws_idx)
        .and_then(|workspace| workspace.tabs.get(tab_idx))
    else {
        return false;
    };
    if tab.zoomed || tab.layout.pane_count() <= 1 {
        return false;
    }

    state.context_menu = None;
    state.pane_layout = Some(PaneLayoutInteractionState {
        ws_idx,
        tab_idx,
        source_pane_id,
        interaction: PaneLayoutInteraction::Preset {
            preset: LayoutPreset::Grid,
        },
    });
    state.mode = Mode::PaneLayout;
    true
}

pub(super) fn open_pane_transfer(
    state: &mut AppState,
    origin: PaneTransferOrigin,
    ws_idx: usize,
    tab_idx: usize,
    source_pane_id: PaneId,
) -> bool {
    let Some(tab) = state
        .workspaces
        .get(ws_idx)
        .and_then(|workspace| workspace.tabs.get(tab_idx))
    else {
        return false;
    };
    if tab.zoomed || !tab.panes.contains_key(&source_pane_id) {
        return false;
    }
    let Some(source) = state.pane_transfer_source(ws_idx, tab_idx, source_pane_id) else {
        return false;
    };

    state.context_menu = None;
    state.pane_layout = Some(PaneLayoutInteractionState {
        ws_idx,
        tab_idx,
        source_pane_id,
        interaction: PaneLayoutInteraction::Transfer(PaneTransferState {
            source,
            origin,
            selected: None,
        }),
    });
    let initial = pane_transfer_candidates(state)
        .into_iter()
        .next()
        .map(|candidate| candidate.destination);
    if let Some(initial) = initial {
        set_transfer_destination(state, initial);
    }
    state.mode = Mode::PaneLayout;
    true
}

pub(crate) fn pane_transfer_candidates(state: &AppState) -> Vec<PaneTransferCandidate> {
    state.pane_transfer_candidates()
}

pub(crate) fn set_transfer_destination(state: &mut AppState, destination: PaneTransferDestination) {
    let Some(PaneLayoutInteractionState {
        interaction: PaneLayoutInteraction::Transfer(transfer),
        ..
    }) = state.pane_layout.as_mut()
    else {
        return;
    };
    transfer.selected = Some(destination);
}

pub(crate) fn cancel_pane_layout(state: &mut AppState) {
    state.pane_layout = None;
    leave_modal(state);
}

pub(crate) fn update_pane_layout_from_mouse(state: &mut AppState, col: u16, row: u16) -> bool {
    let Some(current) = state.pane_layout.clone() else {
        return false;
    };
    match current.interaction {
        PaneLayoutInteraction::Reposition { .. } => {
            let Some(target) = state
                .view
                .pane_infos
                .iter()
                .find(|pane| {
                    pane.id != current.source_pane_id && rect_contains(pane.rect, col, row)
                })
                .cloned()
            else {
                return false;
            };
            let placement = nearest_edge(target.rect, col, row);
            if let Some(layout) = state.pane_layout.as_mut() {
                layout.interaction = PaneLayoutInteraction::Reposition {
                    target_pane_id: target.id,
                    placement,
                };
            }
            true
        }
        PaneLayoutInteraction::Preset { .. } => {
            let Some(preset) = crate::ui::pane_layout_preset_at(state.view.terminal_area, col, row)
            else {
                return false;
            };
            if let Some(layout) = state.pane_layout.as_mut() {
                layout.interaction = PaneLayoutInteraction::Preset { preset };
            }
            true
        }
        PaneLayoutInteraction::Transfer(transfer) => {
            if let Some(destination) =
                crate::ui::pane_transfer_destination_at(state, state.view.terminal_area, col, row)
            {
                set_transfer_destination(state, destination);
                return true;
            }
            let Some(target) = state
                .view
                .pane_infos
                .iter()
                .find(|pane| rect_contains(pane.rect, col, row))
                .cloned()
            else {
                return false;
            };
            let Some(ws_idx) = state.active else {
                return false;
            };
            let Some(tab_idx) = state
                .workspaces
                .get(ws_idx)
                .map(crate::workspace::Workspace::active_tab_index)
            else {
                return false;
            };
            let Some(target_source) = state.pane_transfer_source(ws_idx, tab_idx, target.id) else {
                return false;
            };
            if target_source.pane_id == transfer.source.pane_id {
                return false;
            }
            set_transfer_destination(
                state,
                PaneTransferDestination::PaneEdge {
                    workspace_id: target_source.workspace_id,
                    tab_id: target_source.tab_id,
                    pane_id: target_source.pane_id,
                    placement: nearest_edge(target.rect, col, row),
                },
            );
            true
        }
    }
}

impl App {
    pub(crate) fn handle_pane_layout_key_via_api(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => cancel_pane_layout(&mut self.state),
            KeyCode::Enter => self.commit_pane_layout_via_api(),
            KeyCode::Tab => {
                let delta = if key.modifiers.contains(KeyModifiers::SHIFT) {
                    -1
                } else {
                    1
                };
                cycle_reposition_target(&mut self.state, delta);
                cycle_transfer_target(&mut self.state, delta);
            }
            KeyCode::Left | KeyCode::Char('h') => {
                set_reposition_placement(&mut self.state, PanePlacement::Left);
                set_transfer_placement(&mut self.state, PanePlacement::Left);
                cycle_preset(&mut self.state, -1);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                set_reposition_placement(&mut self.state, PanePlacement::Right);
                set_transfer_placement(&mut self.state, PanePlacement::Right);
                cycle_preset(&mut self.state, 1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                set_reposition_placement(&mut self.state, PanePlacement::Up);
                set_transfer_placement(&mut self.state, PanePlacement::Up);
                cycle_preset(&mut self.state, -1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                set_reposition_placement(&mut self.state, PanePlacement::Down);
                set_transfer_placement(&mut self.state, PanePlacement::Down);
                cycle_preset(&mut self.state, 1);
            }
            KeyCode::Char(value @ '1'..='5') => {
                let idx = usize::from(value as u8 - b'1');
                if let Some(preset) = LayoutPreset::ALL.get(idx).copied() {
                    set_preset(&mut self.state, preset);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn commit_pane_layout_via_api(&mut self) {
        let Some(layout) = self.state.pane_layout.clone() else {
            return;
        };
        if matches!(&layout.interaction, PaneLayoutInteraction::Transfer(_)) {
            self.commit_pane_transfer_via_api();
            return;
        }
        let Some(anchor_pane_id) = self.public_pane_id(layout.ws_idx, layout.source_pane_id) else {
            cancel_pane_layout(&mut self.state);
            return;
        };
        let operation = match layout.interaction {
            PaneLayoutInteraction::Reposition {
                target_pane_id,
                placement,
            } => {
                let Some(target_pane_id) = self.public_pane_id(layout.ws_idx, target_pane_id)
                else {
                    cancel_pane_layout(&mut self.state);
                    return;
                };
                LayoutRearrangeOperation::Reposition {
                    source_pane_id: anchor_pane_id,
                    target_pane_id,
                    placement: placement.into(),
                    ratio: Some(0.5),
                }
            }
            PaneLayoutInteraction::Preset { preset } => LayoutRearrangeOperation::Preset {
                anchor_pane_id,
                preset: preset.into(),
            },
            PaneLayoutInteraction::Transfer(_) => unreachable!("transfer handled above"),
        };
        self.runtime_layout_rearrange(
            "tui.layout.rearrange",
            LayoutRearrangeParams {
                operation,
                focus: true,
            },
        );
        self.state.pane_layout = None;
        leave_modal(&mut self.state);
    }

    pub(crate) fn commit_pane_transfer_via_api(&mut self) {
        let Some(crate::app::state::PaneTransferState {
            source,
            selected: Some(destination),
            ..
        }) = self.state.pane_layout.as_ref().and_then(|layout| {
            let PaneLayoutInteraction::Transfer(transfer) = &layout.interaction else {
                return None;
            };
            Some(transfer.clone())
        })
        else {
            self.finish_pane_transfer(false);
            return;
        };
        let Some((source_ws_idx, source_tab_idx, source_pane_id)) =
            self.state.resolve_pane_transfer_source(&source)
        else {
            self.finish_pane_transfer(false);
            return;
        };
        if self.state.workspaces[source_ws_idx].tabs[source_tab_idx].zoomed {
            self.finish_pane_transfer(false);
            return;
        }

        let changed = match destination {
            PaneTransferDestination::PaneEdge {
                workspace_id,
                tab_id,
                pane_id,
                placement,
            } => {
                let target_source = crate::app::state::PaneTransferSource {
                    workspace_id,
                    tab_id,
                    pane_id,
                };
                let Some((target_ws_idx, target_tab_idx, target_pane_id)) =
                    self.state.resolve_pane_transfer_source(&target_source)
                else {
                    self.finish_pane_transfer(false);
                    return;
                };
                if source_pane_id == target_pane_id
                    || self.state.workspaces[target_ws_idx].tabs[target_tab_idx].zoomed
                {
                    self.finish_pane_transfer(false);
                    return;
                }
                if source_ws_idx == target_ws_idx && source_tab_idx == target_tab_idx {
                    let response = self.runtime_layout_rearrange(
                        "tui.pane.transfer.rearrange",
                        LayoutRearrangeParams {
                            operation: LayoutRearrangeOperation::Reposition {
                                source_pane_id: source.pane_id,
                                target_pane_id: target_source.pane_id,
                                placement: placement.into(),
                                ratio: Some(0.5),
                            },
                            focus: true,
                        },
                    );
                    pane_transfer_response_changed(&response)
                } else {
                    let response = self.runtime_pane_move(
                        "tui.pane.transfer.move",
                        PaneMoveParams {
                            pane_id: source.pane_id,
                            destination: ApiPaneMoveDestination::Tab {
                                tab_id: target_source.tab_id,
                                target_pane_id: Some(target_source.pane_id),
                                split: placement_to_split_direction(placement),
                                ratio: Some(0.5),
                            },
                            focus: true,
                        },
                    );
                    pane_transfer_response_changed(&response)
                }
            }
            PaneTransferDestination::NewTab { workspace_id } => {
                if !self
                    .state
                    .workspaces
                    .iter()
                    .any(|workspace| workspace.id == workspace_id)
                {
                    self.finish_pane_transfer(false);
                    return;
                }
                let response = self.runtime_pane_move(
                    "tui.pane.transfer.new_tab",
                    PaneMoveParams {
                        pane_id: source.pane_id,
                        destination: ApiPaneMoveDestination::NewTab {
                            workspace_id: Some(workspace_id),
                            label: None,
                        },
                        focus: true,
                    },
                );
                pane_transfer_response_changed(&response)
            }
            PaneTransferDestination::NewWorkspace => {
                let source_label = self
                    .state
                    .workspaces
                    .iter()
                    .find(|ws| ws.id == source.workspace_id)
                    .and_then(|ws| ws.custom_name.clone());
                let response = self.runtime_pane_move(
                    "tui.pane.transfer.new_workspace",
                    PaneMoveParams {
                        pane_id: source.pane_id,
                        destination: ApiPaneMoveDestination::NewWorkspace {
                            label: source_label,
                            tab_label: None,
                        },
                        focus: true,
                    },
                );
                pane_transfer_response_changed(&response)
            }
        };
        self.finish_pane_transfer(changed);
    }

    fn finish_pane_transfer(&mut self, changed: bool) {
        let previous_toast = self.state.toast.clone();
        self.state.pane_layout = None;
        leave_modal(&mut self.state);
        if changed {
            return;
        }
        self.state.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::NeedsAttention,
            title: t!("state.pane_transfer_failed").to_string(),
            context: String::new(),
            position: None,
            target: None,
        });
        self.sync_toast_deadline(previous_toast);
    }
}

pub(crate) fn placement_to_split_direction(placement: PanePlacement) -> SplitDirection {
    match placement {
        PanePlacement::Left | PanePlacement::Right => SplitDirection::Right,
        PanePlacement::Up | PanePlacement::Down => SplitDirection::Down,
    }
}

fn pane_transfer_response_changed(response: &str) -> bool {
    let Ok(success) = serde_json::from_str::<SuccessResponse>(response) else {
        return false;
    };
    match success.result {
        ResponseResult::LayoutRearrange { rearrange } => rearrange.changed,
        ResponseResult::PaneMove { move_result } => move_result.changed,
        _ => false,
    }
}

fn visual_pane_order(state: &AppState, ws_idx: usize, tab_idx: usize) -> Vec<PaneId> {
    let Some(tab) = state
        .workspaces
        .get(ws_idx)
        .and_then(|workspace| workspace.tabs.get(tab_idx))
    else {
        return Vec::new();
    };
    let mut panes = tab.layout.panes(state.view.terminal_area);
    panes.sort_by_key(|pane| (pane.rect.y, pane.rect.x));
    panes.into_iter().map(|pane| pane.id).collect()
}

fn cycle_reposition_target(state: &mut AppState, delta: isize) {
    let Some(current) = state.pane_layout.clone() else {
        return;
    };
    let PaneLayoutInteraction::Reposition {
        target_pane_id,
        placement,
    } = current.interaction
    else {
        return;
    };
    let panes = visual_pane_order(state, current.ws_idx, current.tab_idx)
        .into_iter()
        .filter(|pane_id| *pane_id != current.source_pane_id)
        .collect::<Vec<_>>();
    let Some(current_idx) = panes.iter().position(|pane_id| *pane_id == target_pane_id) else {
        return;
    };
    let next_idx = wrap_index(current_idx, delta, panes.len());
    if let Some(layout) = state.pane_layout.as_mut() {
        layout.interaction = PaneLayoutInteraction::Reposition {
            target_pane_id: panes[next_idx],
            placement,
        };
    }
}

fn cycle_transfer_target(state: &mut AppState, delta: isize) {
    let candidates = pane_transfer_candidates(state);
    if candidates.is_empty() {
        return;
    }
    let selected = state.pane_layout.as_ref().and_then(|layout| {
        let PaneLayoutInteraction::Transfer(transfer) = &layout.interaction else {
            return None;
        };
        transfer.selected.as_ref()
    });
    let current_idx = selected
        .and_then(|selected| {
            candidates
                .iter()
                .position(|candidate| &candidate.destination == selected)
        })
        .unwrap_or(0);
    let next_idx = wrap_index(current_idx, delta, candidates.len());
    set_transfer_destination(state, candidates[next_idx].destination.clone());
}

fn set_reposition_placement(state: &mut AppState, placement: PanePlacement) {
    let Some(layout) = state.pane_layout.as_mut() else {
        return;
    };
    let PaneLayoutInteraction::Reposition { target_pane_id, .. } = &layout.interaction else {
        return;
    };
    layout.interaction = PaneLayoutInteraction::Reposition {
        target_pane_id: *target_pane_id,
        placement,
    };
}

fn set_transfer_placement(state: &mut AppState, placement: PanePlacement) {
    let Some(PaneLayoutInteractionState {
        interaction: PaneLayoutInteraction::Transfer(transfer),
        ..
    }) = state.pane_layout.as_mut()
    else {
        return;
    };
    let next = match transfer.selected.as_ref() {
        Some(PaneTransferDestination::PaneEdge {
            workspace_id,
            tab_id,
            pane_id,
            ..
        }) => PaneTransferDestination::PaneEdge {
            workspace_id: workspace_id.clone(),
            tab_id: tab_id.clone(),
            pane_id: pane_id.clone(),
            placement,
        },
        Some(PaneTransferDestination::NewTab { .. } | PaneTransferDestination::NewWorkspace)
        | None => return,
    };
    transfer.selected = Some(next);
}

fn cycle_preset(state: &mut AppState, delta: isize) {
    let Some(layout) = state.pane_layout.clone() else {
        return;
    };
    let PaneLayoutInteraction::Preset { preset } = layout.interaction else {
        return;
    };
    let Some(current_idx) = LayoutPreset::ALL
        .iter()
        .position(|candidate| *candidate == preset)
    else {
        return;
    };
    set_preset(
        state,
        LayoutPreset::ALL[wrap_index(current_idx, delta, LayoutPreset::ALL.len())],
    );
}

fn set_preset(state: &mut AppState, preset: LayoutPreset) {
    let Some(layout) = state.pane_layout.as_mut() else {
        return;
    };
    if matches!(&layout.interaction, PaneLayoutInteraction::Preset { .. }) {
        layout.interaction = PaneLayoutInteraction::Preset { preset };
    }
}

fn wrap_index(current: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (current as isize + delta).rem_euclid(len as isize) as usize
}

fn rect_contains(rect: ratatui::layout::Rect, col: u16, row: u16) -> bool {
    rect.width > 0
        && rect.height > 0
        && col >= rect.x
        && col < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

fn nearest_edge(rect: ratatui::layout::Rect, col: u16, row: u16) -> PanePlacement {
    let width = rect.width.max(1) as f32;
    let height = rect.height.max(1) as f32;
    let left = col.saturating_sub(rect.x) as f32 / width;
    let right = rect
        .x
        .saturating_add(rect.width.saturating_sub(1))
        .saturating_sub(col) as f32
        / width;
    let up = row.saturating_sub(rect.y) as f32 / height;
    let down = rect
        .y
        .saturating_add(rect.height.saturating_sub(1))
        .saturating_sub(row) as f32
        / height;
    [
        (left, PanePlacement::Left),
        (right, PanePlacement::Right),
        (up, PanePlacement::Up),
        (down, PanePlacement::Down),
    ]
    .into_iter()
    .min_by(|(lhs, _), (rhs, _)| lhs.total_cmp(rhs))
    .map(|(_, placement)| placement)
    .unwrap_or(PanePlacement::Right)
}

impl From<PanePlacement> for PaneDirection {
    fn from(placement: PanePlacement) -> Self {
        match placement {
            PanePlacement::Left => Self::Left,
            PanePlacement::Right => Self::Right,
            PanePlacement::Up => Self::Up,
            PanePlacement::Down => Self::Down,
        }
    }
}

impl From<LayoutPreset> for PaneLayoutPreset {
    fn from(preset: LayoutPreset) -> Self {
        match preset {
            LayoutPreset::Columns => Self::Columns,
            LayoutPreset::Rows => Self::Rows,
            LayoutPreset::Grid => Self::Grid,
            LayoutPreset::MainLeft => Self::MainLeft,
            LayoutPreset::MainTop => Self::MainTop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{api::schema::EventData, workspace::Workspace};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::layout::{Direction, Rect};

    fn app_with_three_panes() -> (App, PaneId, PaneId, PaneId) {
        let mut app = super::super::app_for_mouse_test();
        let mut workspace = Workspace::test_new("layout");
        let root = workspace.tabs[0].root_pane;
        let second = workspace.test_split(Direction::Horizontal);
        let third = workspace.test_split(Direction::Vertical);
        workspace.tabs[0].layout.focus_pane(root);
        app.state.workspaces = vec![workspace];
        app.state.ensure_test_terminals();
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 30));
        (app, root, second, third)
    }

    #[test]
    fn pane_transfer_candidates_use_public_identity_and_preview_is_pure() {
        let (mut app, source, target, _) = app_with_three_panes();
        let source_public = app.public_pane_id(0, source).expect("source public id");
        let before = app.state.workspaces[0].tabs[0].layout.clone();

        assert!(open_pane_transfer(
            &mut app.state,
            PaneTransferOrigin::ContextMenu,
            0,
            0,
            source,
        ));
        let transfer = app.state.pane_layout.as_ref().expect("transfer");
        assert_eq!(
            transfer.transfer_source().expect("source").pane_id,
            source_public
        );
        assert!(pane_transfer_candidates(&app.state)
            .iter()
            .any(|candidate| {
                matches!(
                    &candidate.destination,
                    PaneTransferDestination::PaneEdge { pane_id, .. }
                        if app.parse_pane_id(pane_id) == Some((0, target))
                )
            }));
        assert!(app.state.pane_transfer_preview().is_some());
        assert_eq!(app.state.workspaces[0].tabs[0].layout, before);
    }

    #[test]
    fn transfer_direction_updates_selected_session_without_duplicate_rows() {
        let (mut app, source, target, _) = app_with_three_panes();
        let target_public = app.public_pane_id(0, target).expect("target public id");
        assert!(open_pane_transfer(
            &mut app.state,
            PaneTransferOrigin::ContextMenu,
            0,
            0,
            source,
        ));
        let target_destination = pane_transfer_candidates(&app.state)
            .into_iter()
            .find(|candidate| {
                matches!(
                    &candidate.destination,
                    PaneTransferDestination::PaneEdge { pane_id, .. }
                        if pane_id == &target_public
                )
            })
            .map(|candidate| candidate.destination)
            .expect("target session");
        set_transfer_destination(&mut app.state, target_destination);

        set_transfer_placement(&mut app.state, PanePlacement::Up);

        let target_rows = pane_transfer_candidates(&app.state)
            .into_iter()
            .filter(|candidate| {
                matches!(
                    &candidate.destination,
                    PaneTransferDestination::PaneEdge { pane_id, .. }
                        if pane_id == &target_public
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            target_rows.len(),
            1,
            "changing the split edge must not duplicate the target session"
        );
        assert!(matches!(
            target_rows[0].destination,
            PaneTransferDestination::PaneEdge {
                placement: PanePlacement::Up,
                ..
            }
        ));
    }

    #[test]
    fn cross_tab_transfer_preview_does_not_switch_live_context() {
        let (mut app, source, _, _) = app_with_three_panes();
        let target_tab_idx = app.state.workspaces[0].test_add_tab(Some("target"));
        let target = app.state.workspaces[0].tabs[target_tab_idx].root_pane;
        app.state.ensure_test_terminals();
        let source_layout = app.state.workspaces[0].tabs[0].layout.clone();
        let target_layout = app.state.workspaces[0].tabs[target_tab_idx].layout.clone();
        let before_active_workspace = app.state.active;
        let before_active_tab = app.state.workspaces[0].active_tab;
        let target_workspace_id = app.public_workspace_id(0);
        let target_tab_id = app.public_tab_id(0, target_tab_idx).expect("target tab id");
        let target_pane_id = app.public_pane_id(0, target).expect("target pane id");

        assert!(open_pane_transfer(
            &mut app.state,
            PaneTransferOrigin::ContextMenu,
            0,
            0,
            source,
        ));
        set_transfer_destination(
            &mut app.state,
            PaneTransferDestination::PaneEdge {
                workspace_id: target_workspace_id,
                tab_id: target_tab_id,
                pane_id: target_pane_id,
                placement: PanePlacement::Right,
            },
        );

        let preview = app
            .state
            .pane_transfer_preview()
            .expect("cross-tab transfer preview");
        assert!(preview
            .iter()
            .any(|pane| pane.role == PreviewPaneRole::Source));
        assert!(preview
            .iter()
            .any(|pane| pane.role == PreviewPaneRole::Target));
        assert_eq!(app.state.active, before_active_workspace);
        assert_eq!(app.state.workspaces[0].active_tab, before_active_tab);
        assert_eq!(app.state.workspaces[0].tabs[0].layout, source_layout);
        assert_eq!(
            app.state.workspaces[0].tabs[target_tab_idx].layout,
            target_layout
        );
    }

    #[test]
    fn transfer_commit_rearranges_same_tab_without_moving_runtime() {
        let (mut app, source, target, _) = app_with_three_panes();
        let source_terminal = app.state.workspaces[0].tabs[0]
            .terminal_id(source)
            .expect("source terminal")
            .clone();
        assert!(open_pane_transfer(
            &mut app.state,
            PaneTransferOrigin::ContextMenu,
            0,
            0,
            source,
        ));
        let destination = PaneTransferDestination::PaneEdge {
            workspace_id: app.public_workspace_id(0),
            tab_id: app.public_tab_id(0, 0).expect("tab id"),
            pane_id: app.public_pane_id(0, target).expect("target id"),
            placement: PanePlacement::Left,
        };
        set_transfer_destination(&mut app.state, destination);

        app.commit_pane_transfer_via_api();

        assert_eq!(
            app.state.workspaces[0].tabs[0].terminal_id(source),
            Some(&source_terminal)
        );
        let panes = app.state.workspaces[0].tabs[0]
            .layout
            .panes(Rect::new(0, 0, 80, 24));
        let source_rect = panes
            .iter()
            .find(|pane| pane.id == source)
            .expect("source")
            .rect;
        let target_rect = panes
            .iter()
            .find(|pane| pane.id == target)
            .expect("target")
            .rect;
        assert!(source_rect.x < target_rect.x);
        assert!(app.state.pane_layout.is_none());
    }

    #[test]
    fn transfer_commit_moves_existing_runtime_across_workspaces() {
        let (mut app, source, _, _) = app_with_three_panes();
        app.state.workspaces.push(Workspace::test_new("target"));
        app.state.ensure_test_terminals();
        let source_terminal = app.state.workspaces[0].tabs[0]
            .terminal_id(source)
            .expect("source terminal")
            .clone();
        let source_public = app.public_pane_id(0, source).expect("source public");
        let target = app.state.workspaces[1].tabs[0].root_pane;
        assert!(open_pane_transfer(
            &mut app.state,
            PaneTransferOrigin::ContextMenu,
            0,
            0,
            source,
        ));
        let destination = PaneTransferDestination::PaneEdge {
            workspace_id: app.public_workspace_id(1),
            tab_id: app.public_tab_id(1, 0).expect("target tab"),
            pane_id: app.public_pane_id(1, target).expect("target pane"),
            placement: PanePlacement::Right,
        };
        set_transfer_destination(&mut app.state, destination);

        app.commit_pane_transfer_via_api();

        let (ws_idx, pane_id) = app
            .parse_pane_id(&source_public)
            .expect("old alias resolves");
        assert_eq!(ws_idx, 1);
        assert_eq!(
            app.state.workspaces[ws_idx].tabs[0].terminal_id(pane_id),
            Some(&source_terminal)
        );
        assert!(app.state.pane_layout.is_none());
    }

    #[test]
    fn transfer_commit_detaches_existing_runtime_to_new_tab() {
        let (mut app, source, _, _) = app_with_three_panes();
        let source_terminal = app.state.workspaces[0].tabs[0]
            .terminal_id(source)
            .expect("source terminal")
            .clone();
        let source_public = app.public_pane_id(0, source).expect("source public");
        assert!(open_pane_transfer(
            &mut app.state,
            PaneTransferOrigin::ContextMenu,
            0,
            0,
            source,
        ));
        let workspace_id = app.public_workspace_id(0);
        set_transfer_destination(
            &mut app.state,
            PaneTransferDestination::NewTab { workspace_id },
        );

        app.commit_pane_transfer_via_api();

        let (ws_idx, pane_id) = app
            .parse_pane_id(&source_public)
            .expect("old alias resolves");
        let tab_idx = app.state.workspaces[ws_idx]
            .find_tab_index_for_pane(pane_id)
            .expect("moved pane tab");
        assert_ne!(tab_idx, 0);
        assert_eq!(
            app.state.workspaces[ws_idx].tabs[tab_idx].terminal_id(pane_id),
            Some(&source_terminal)
        );
        assert_eq!(app.state.workspaces[ws_idx].tabs.len(), 2);
    }

    #[test]
    fn transfer_commit_detaches_existing_runtime_to_new_workspace() {
        let (mut app, source, _, _) = app_with_three_panes();
        let source_terminal = app.state.workspaces[0].tabs[0]
            .terminal_id(source)
            .expect("source terminal")
            .clone();
        let source_public = app.public_pane_id(0, source).expect("source public");
        assert!(open_pane_transfer(
            &mut app.state,
            PaneTransferOrigin::ContextMenu,
            0,
            0,
            source,
        ));
        set_transfer_destination(&mut app.state, PaneTransferDestination::NewWorkspace);

        app.commit_pane_transfer_via_api();

        let (ws_idx, pane_id) = app
            .parse_pane_id(&source_public)
            .expect("old alias resolves");
        assert_eq!(ws_idx, 1);
        assert_eq!(
            app.state.workspaces[ws_idx].tabs[0].terminal_id(pane_id),
            Some(&source_terminal)
        );
        assert_eq!(app.state.workspaces.len(), 2);
    }

    #[test]
    fn transfer_commit_rejects_stale_target_without_changing_source_layout() {
        let (mut app, source, _, _) = app_with_three_panes();
        let before = app.state.workspaces[0].tabs[0].layout.clone();
        assert!(open_pane_transfer(
            &mut app.state,
            PaneTransferOrigin::ContextMenu,
            0,
            0,
            source,
        ));
        let workspace_id = app.public_workspace_id(0);
        let tab_id = app.public_tab_id(0, 0).expect("tab id");
        set_transfer_destination(
            &mut app.state,
            PaneTransferDestination::PaneEdge {
                workspace_id,
                tab_id,
                pane_id: "missing:pane".into(),
                placement: PanePlacement::Right,
            },
        );

        app.commit_pane_transfer_via_api();

        assert_eq!(app.state.workspaces[0].tabs[0].layout, before);
        assert!(app.state.pane_layout.is_none());
        assert_eq!(
            app.state.toast.as_ref().map(|toast| toast.title.as_str()),
            Some(t!("state.pane_transfer_failed").as_ref())
        );
    }

    #[test]
    fn transfer_commit_rejects_zoomed_target_without_changing_source_layout() {
        let (mut app, source, _, _) = app_with_three_panes();
        app.state.workspaces.push(Workspace::test_new("target"));
        app.state.ensure_test_terminals();
        let before = app.state.workspaces[0].tabs[0].layout.clone();
        let target = app.state.workspaces[1].tabs[0].root_pane;
        assert!(open_pane_transfer(
            &mut app.state,
            PaneTransferOrigin::ContextMenu,
            0,
            0,
            source,
        ));
        let workspace_id = app.public_workspace_id(1);
        let tab_id = app.public_tab_id(1, 0).expect("target tab");
        let pane_id = app.public_pane_id(1, target).expect("target pane");
        set_transfer_destination(
            &mut app.state,
            PaneTransferDestination::PaneEdge {
                workspace_id,
                tab_id,
                pane_id,
                placement: PanePlacement::Right,
            },
        );
        app.state.workspaces[1].tabs[0].zoomed = true;

        app.commit_pane_transfer_via_api();

        assert_eq!(app.state.workspaces[0].tabs[0].layout, before);
        assert!(app.state.workspaces[0].tabs[0].panes.contains_key(&source));
        assert!(app.state.pane_layout.is_none());
    }

    #[test]
    fn reposition_preview_uses_clicked_source_without_mutating_live_layout() {
        let (mut app, root, second, source) = app_with_three_panes();
        let before = app.state.workspaces[0].tabs[0].layout.clone();

        assert!(open_pane_reposition(&mut app.state, 0, 0, source));
        assert_eq!(app.state.mode, Mode::PaneLayout);
        assert!(matches!(
            app.state.pane_layout,
            Some(PaneLayoutInteractionState {
                source_pane_id,
                interaction: PaneLayoutInteraction::Reposition {
                    target_pane_id,
                    placement: PanePlacement::Right,
                },
                ..
            }) if source_pane_id == source && target_pane_id == root
        ));
        assert_ne!(app.state.pane_layout_preview().unwrap(), before);
        assert_eq!(app.state.workspaces[0].tabs[0].layout, before);

        cycle_reposition_target(&mut app.state, 1);
        set_reposition_placement(&mut app.state, PanePlacement::Up);
        assert!(matches!(
            app.state.pane_layout,
            Some(PaneLayoutInteractionState {
                interaction: PaneLayoutInteraction::Reposition {
                    target_pane_id,
                    placement: PanePlacement::Up,
                },
                ..
            }) if target_pane_id == second
        ));

        app.handle_pane_layout_key_via_api(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert_eq!(app.state.mode, Mode::Terminal);
        assert!(app.state.pane_layout.is_none());
        assert_eq!(app.state.workspaces[0].tabs[0].layout, before);
    }

    #[test]
    fn preset_commit_reuses_panes_and_terminal_states_then_focuses_anchor() {
        let (mut app, root, second, anchor) = app_with_three_panes();
        let before_layout = app.state.workspaces[0].tabs[0].layout.clone();
        let pane_terminals_before = app.state.workspaces[0].tabs[0]
            .panes
            .iter()
            .map(|(pane_id, pane)| (*pane_id, pane.attached_terminal_id.clone()))
            .collect::<std::collections::HashMap<_, _>>();

        assert!(open_pane_layout_presets(&mut app.state, 0, 0, anchor));
        app.handle_pane_layout_key_via_api(KeyEvent::new(
            KeyCode::Char('4'),
            KeyModifiers::empty(),
        ));
        assert!(matches!(
            app.state.pane_layout,
            Some(PaneLayoutInteractionState {
                source_pane_id,
                interaction: PaneLayoutInteraction::Preset {
                    preset: LayoutPreset::MainLeft,
                },
                ..
            }) if source_pane_id == anchor
        ));

        app.handle_pane_layout_key_via_api(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

        assert_eq!(app.state.mode, Mode::Terminal);
        assert!(app.state.pane_layout.is_none());
        assert_ne!(app.state.workspaces[0].tabs[0].layout, before_layout);
        assert_eq!(app.state.workspaces[0].focused_pane_id(), Some(anchor));
        assert_eq!(
            app.state.workspaces[0].tabs[0]
                .panes
                .iter()
                .map(|(pane_id, pane)| (*pane_id, pane.attached_terminal_id.clone()))
                .collect::<std::collections::HashMap<_, _>>(),
            pane_terminals_before
        );
        assert_eq!(
            app.state.workspaces[0].tabs[0]
                .layout
                .pane_ids()
                .into_iter()
                .collect::<std::collections::HashSet<_>>(),
            [root, second, anchor]
                .into_iter()
                .collect::<std::collections::HashSet<_>>()
        );
        assert!(matches!(
            &app.event_hub.events_after(0).last().expect("layout event").1.data,
            EventData::LayoutUpdated { layout }
                if layout.focused_pane_id == app.public_pane_id(0, anchor).unwrap()
        ));
    }

    #[test]
    fn layout_interaction_is_unavailable_for_zoomed_or_single_pane_tabs() {
        let mut state = AppState::test_new();
        state.workspaces = vec![Workspace::test_new("single")];
        state.active = Some(0);
        state.mode = Mode::Terminal;
        let root = state.workspaces[0].tabs[0].root_pane;

        assert!(!open_pane_reposition(&mut state, 0, 0, root));
        assert!(!open_pane_layout_presets(&mut state, 0, 0, root));

        let second = state.workspaces[0].test_split(Direction::Horizontal);
        state.workspaces[0].tabs[0].zoomed = true;
        assert!(!open_pane_reposition(&mut state, 0, 0, second));
        assert!(!open_pane_layout_presets(&mut state, 0, 0, second));
        assert_eq!(state.mode, Mode::Terminal);
        assert!(state.pane_layout.is_none());
    }

    #[test]
    fn nearest_edge_uses_normalized_distance_for_asymmetric_panes() {
        let rect = Rect::new(10, 5, 40, 10);
        assert_eq!(nearest_edge(rect, 10, 9), PanePlacement::Left);
        assert_eq!(nearest_edge(rect, 49, 9), PanePlacement::Right);
        assert_eq!(nearest_edge(rect, 30, 5), PanePlacement::Up);
        assert_eq!(nearest_edge(rect, 30, 14), PanePlacement::Down);
    }

    #[test]
    fn mouse_hover_selects_a_target_edge_or_template_without_committing() {
        let (mut app, _root, target, source) = app_with_three_panes();
        let before = app.state.workspaces[0].tabs[0].layout.clone();
        let target_rect = app
            .state
            .view
            .pane_infos
            .iter()
            .find(|pane| pane.id == target)
            .expect("target pane")
            .rect;

        assert!(open_pane_reposition(&mut app.state, 0, 0, source));
        assert!(update_pane_layout_from_mouse(
            &mut app.state,
            target_rect.x,
            target_rect.y + target_rect.height / 2,
        ));
        assert!(matches!(
            app.state.pane_layout,
            Some(PaneLayoutInteractionState {
                interaction: PaneLayoutInteraction::Reposition {
                    target_pane_id,
                    placement: PanePlacement::Left,
                },
                ..
            }) if target_pane_id == target
        ));
        assert_eq!(app.state.workspaces[0].tabs[0].layout, before);

        cancel_pane_layout(&mut app.state);
        assert!(open_pane_layout_presets(&mut app.state, 0, 0, source));
        let area = app.state.view.terminal_area;
        assert!(update_pane_layout_from_mouse(
            &mut app.state,
            area.x + area.width.saturating_sub(2),
            area.y + 5,
        ));
        assert!(matches!(
            app.state.pane_layout,
            Some(PaneLayoutInteractionState {
                interaction: PaneLayoutInteraction::Preset {
                    preset: LayoutPreset::MainTop,
                },
                ..
            })
        ));
        assert_eq!(app.state.workspaces[0].tabs[0].layout, before);
    }
}
