use super::*;

impl ClientPaneMoveOverlay {
    pub(super) fn entries(
        &self,
        snapshot: &ClientShellSnapshot,
    ) -> Vec<(String, ClientPaneMoveEntry)> {
        match self.mode {
            ClientPaneMoveMode::Move => self.move_entries(snapshot),
            ClientPaneMoveMode::Reposition => self.reposition_entries(snapshot),
            ClientPaneMoveMode::Preset => Self::preset_entries(),
        }
    }

    fn move_entries(&self, snapshot: &ClientShellSnapshot) -> Vec<(String, ClientPaneMoveEntry)> {
        let other_tabs = snapshot
            .tabs
            .iter()
            .filter(|tab| tab.tab_id != self.source_tab_id)
            .count();
        let mut entries = Vec::with_capacity(2 + other_tabs);
        entries.push((
            rust_i18n::t!("state.pane_transfer_new_tab").to_string(),
            ClientPaneMoveEntry::NewTab,
        ));
        entries.push((
            rust_i18n::t!("state.pane_transfer_new_workspace").to_string(),
            ClientPaneMoveEntry::NewWorkspace,
        ));
        let workspace_label = |workspace_id: &str| {
            snapshot
                .workspaces
                .iter()
                .find(|workspace| workspace.workspace_id == workspace_id)
                .map(|workspace| workspace.label.clone())
                .unwrap_or_else(|| workspace_id.to_string())
        };
        let mut tabs: Vec<_> = snapshot
            .tabs
            .iter()
            .filter(|tab| tab.tab_id != self.source_tab_id)
            .collect();
        tabs.sort_by_key(|tab| {
            (
                snapshot
                    .workspaces
                    .iter()
                    .find(|workspace| workspace.workspace_id == tab.workspace_id)
                    .map(|workspace| workspace.number)
                    .unwrap_or(usize::MAX),
                tab.number,
            )
        });
        for tab in tabs {
            let label = rust_i18n::t!(
                "state.pane_move_to_tab",
                tab = format!("{}/{}", workspace_label(&tab.workspace_id), tab.label)
            )
            .to_string();
            entries.push((
                label,
                ClientPaneMoveEntry::ExistingTab {
                    tab_id: tab.tab_id.clone(),
                },
            ));
        }
        entries
    }

    fn pane_display_name(
        snapshot: &ClientShellSnapshot,
        pane: &crate::protocol::ClientShellPane,
    ) -> String {
        let workspace_label = snapshot
            .workspaces
            .iter()
            .find(|workspace| workspace.workspace_id == pane.workspace_id)
            .map(|workspace| workspace.label.clone())
            .unwrap_or_else(|| pane.workspace_id.clone());
        let agent_name = snapshot
            .agents
            .iter()
            .find(|agent| agent.pane_id == pane.pane_id)
            .and_then(|agent| {
                agent
                    .display_agent
                    .clone()
                    .or_else(|| agent.agent.clone())
                    .or_else(|| agent.name.clone())
                    .or_else(|| agent.title.clone())
                    .or_else(|| agent.terminal_title_stripped.clone())
            })
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty());
        let short_id = pane
            .pane_id
            .rsplit(':')
            .next()
            .unwrap_or(pane.pane_id.as_str());
        let core = pane
            .label
            .clone()
            .map(|label| label.trim().to_string())
            .filter(|label| !label.is_empty())
            .unwrap_or_else(|| short_id.to_string());
        match agent_name {
            Some(agent) if core != agent => {
                format!("{workspace_label} · {agent} · {core}")
            }
            Some(agent) => format!("{workspace_label} · {agent}"),
            None => {
                let cwd_base = pane
                    .foreground_cwd
                    .clone()
                    .or_else(|| pane.cwd.clone())
                    .and_then(|cwd| {
                        cwd.rsplit('/')
                            .next()
                            .map(|base| base.trim().to_string())
                            .filter(|base| !base.is_empty() && *base != core)
                    });
                match cwd_base {
                    Some(base) => format!("{workspace_label} · {core} · {base}"),
                    None => format!("{workspace_label} · {core}"),
                }
            }
        }
    }

    fn reposition_entries(
        &self,
        snapshot: &ClientShellSnapshot,
    ) -> Vec<(String, ClientPaneMoveEntry)> {
        let mut targets: Vec<_> = snapshot
            .panes
            .iter()
            .filter(|pane| pane.tab_id == self.source_tab_id && pane.pane_id != self.source_pane_id)
            .collect();
        targets.sort_by_key(|pane| pane.pane_id.clone());
        let mut entries = Vec::with_capacity(targets.len());
        for target in targets {
            let pane_label = Self::pane_display_name(snapshot, target);
            let dir = rust_i18n::t!("state.pane_dir_right").to_string();
            let label = rust_i18n::t!(
                "state.pane_reposition_to",
                dir = dir,
                pane = pane_label.clone()
            )
            .to_string();
            entries.push((
                label,
                ClientPaneMoveEntry::Reposition {
                    target_pane_id: target.pane_id.clone(),
                    placement: crate::api::schema::PaneDirection::Right,
                },
            ));
        }
        entries
    }

    fn preset_entries() -> Vec<(String, ClientPaneMoveEntry)> {
        use crate::api::schema::PaneLayoutPreset as Preset;
        [
            (
                rust_i18n::t!("state.layout_preset_grid").to_string(),
                Preset::Grid,
            ),
            (
                rust_i18n::t!("state.layout_preset_columns").to_string(),
                Preset::Columns,
            ),
            (
                rust_i18n::t!("state.layout_preset_rows").to_string(),
                Preset::Rows,
            ),
            (
                rust_i18n::t!("state.layout_preset_main_left").to_string(),
                Preset::MainLeft,
            ),
            (
                rust_i18n::t!("state.layout_preset_main_top").to_string(),
                Preset::MainTop,
            ),
        ]
        .into_iter()
        .map(|(label, preset)| (label, ClientPaneMoveEntry::Preset { preset }))
        .collect()
    }

    pub(super) fn title(&self) -> String {
        match self.mode {
            ClientPaneMoveMode::Move => rust_i18n::t!("state.pane_transfer_title").to_string(),
            ClientPaneMoveMode::Reposition => rust_i18n::t!("state.layout_move_title").to_string(),
            ClientPaneMoveMode::Preset => rust_i18n::t!("state.layout_templates_title").to_string(),
        }
    }

    pub(super) fn entry_count(&self, snapshot: &ClientShellSnapshot) -> usize {
        match self.mode {
            ClientPaneMoveMode::Move => {
                2 + snapshot
                    .tabs
                    .iter()
                    .filter(|tab| tab.tab_id != self.source_tab_id)
                    .count()
            }
            ClientPaneMoveMode::Reposition => snapshot
                .panes
                .iter()
                .filter(|pane| {
                    pane.tab_id == self.source_tab_id && pane.pane_id != self.source_pane_id
                })
                .count(),
            ClientPaneMoveMode::Preset => 5,
        }
    }

    pub(super) fn hint(&self) -> String {
        match self.mode {
            ClientPaneMoveMode::Preset => rust_i18n::t!("state.pane_preset_hint").to_string(),
            _ => rust_i18n::t!("state.pane_pick_hint").to_string(),
        }
    }
}

impl ClientShellState {
    pub(super) fn open_pane_move_overlay(
        &mut self,
        source_pane_id: String,
        source_tab_id: String,
        mode: ClientPaneMoveMode,
    ) {
        self.overlay = Some(ClientShellOverlay::PaneMove(ClientPaneMoveOverlay {
            source_pane_id,
            source_tab_id,
            mode,
            selected: 0,
        }));
    }

    pub(super) fn move_pane_move_selection(&mut self, delta: i32) {
        let count = self
            .snapshot
            .as_deref()
            .zip(self.overlay.as_ref())
            .and_then(|(snapshot, overlay)| match overlay {
                ClientShellOverlay::PaneMove(pane_move) => Some(pane_move.entry_count(snapshot)),
                _ => None,
            })
            .unwrap_or(0);
        if count == 0 {
            return;
        }
        if let Some(ClientShellOverlay::PaneMove(pane_move)) = self.overlay.as_mut() {
            let next = pane_move.selected as i64 + delta as i64;
            pane_move.selected = next.rem_euclid(count as i64) as usize;
        }
    }

    pub(super) fn move_pane_move_selection_clamped(&mut self, delta: i32) {
        let count = self
            .snapshot
            .as_deref()
            .zip(self.overlay.as_ref())
            .and_then(|(snapshot, overlay)| match overlay {
                ClientShellOverlay::PaneMove(pane_move) => Some(pane_move.entry_count(snapshot)),
                _ => None,
            })
            .unwrap_or(0);
        if count == 0 {
            return;
        }
        if let Some(ClientShellOverlay::PaneMove(pane_move)) = self.overlay.as_mut() {
            let next = pane_move.selected as i64 + delta as i64;
            pane_move.selected = next.clamp(0, count as i64 - 1) as usize;
        }
    }

    pub(super) fn submit_pane_move(&mut self, outcome: &mut ClientShellInput) {
        use crate::api::schema::{
            LayoutRearrangeOperation, LayoutRearrangeParams, Method, PaneMoveDestination,
            PaneMoveParams, SplitDirection,
        };
        let (source_pane_id, entry) = match (self.snapshot.as_deref(), self.overlay.as_ref()) {
            (Some(snapshot), Some(ClientShellOverlay::PaneMove(pane_move))) => {
                let entries = pane_move.entries(snapshot);
                if entries.is_empty() {
                    self.endpoint_error = Some(rust_i18n::t!("state.pane_move_empty").to_string());
                    outcome.repaint = true;
                    return;
                }
                match entries.get(pane_move.selected) {
                    Some((_, entry)) => (pane_move.source_pane_id.clone(), entry.clone()),
                    None => return,
                }
            }
            _ => return,
        };
        let method = match entry {
            ClientPaneMoveEntry::NewTab => Method::PaneMove(PaneMoveParams {
                pane_id: source_pane_id,
                destination: PaneMoveDestination::NewTab {
                    workspace_id: None,
                    label: None,
                },
                focus: true,
            }),
            ClientPaneMoveEntry::NewWorkspace => Method::PaneMove(PaneMoveParams {
                pane_id: source_pane_id,
                destination: PaneMoveDestination::NewWorkspace {
                    label: None,
                    tab_label: None,
                },
                focus: true,
            }),
            ClientPaneMoveEntry::ExistingTab { tab_id } => Method::PaneMove(PaneMoveParams {
                pane_id: source_pane_id,
                destination: PaneMoveDestination::Tab {
                    tab_id,
                    target_pane_id: None,
                    split: SplitDirection::Right,
                    ratio: None,
                },
                focus: true,
            }),
            ClientPaneMoveEntry::Reposition {
                target_pane_id,
                placement,
            } => Method::LayoutRearrange(LayoutRearrangeParams {
                operation: LayoutRearrangeOperation::Reposition {
                    source_pane_id,
                    target_pane_id,
                    placement,
                    ratio: None,
                },
                focus: true,
            }),
            ClientPaneMoveEntry::Preset { preset } => {
                Method::LayoutRearrange(LayoutRearrangeParams {
                    operation: LayoutRearrangeOperation::Preset {
                        anchor_pane_id: source_pane_id,
                        preset,
                    },
                    focus: true,
                })
            }
        };
        self.overlay = None;
        self.push_endpoint_method(method, outcome);
        outcome.repaint = true;
    }

    pub(super) fn route_pane_move_key(
        &mut self,
        key: &crate::input::TerminalKey,
        outcome: &mut ClientShellInput,
    ) -> bool {
        if !matches!(self.overlay, Some(ClientShellOverlay::PaneMove(_))) {
            return false;
        }
        let (code, modifiers) = crate::config::normalize_key_combo((key.code, key.modifiers));
        match code {
            KeyCode::Esc => {
                self.overlay = None;
                outcome.repaint = true;
            }
            KeyCode::Enter => self.submit_pane_move(outcome),
            KeyCode::Up => {
                self.move_pane_move_selection(-1);
                outcome.repaint = true;
            }
            KeyCode::Down => {
                self.move_pane_move_selection(1);
                outcome.repaint = true;
            }
            KeyCode::Char(character)
                if modifiers.is_empty() && character.is_ascii_digit() && character != '0' =>
            {
                let index = (character as u8 - b'1') as usize;
                let count = self
                    .snapshot
                    .as_deref()
                    .zip(self.overlay.as_ref())
                    .and_then(|(snapshot, overlay)| match overlay {
                        ClientShellOverlay::PaneMove(pane_move) => {
                            Some(pane_move.entry_count(snapshot))
                        }
                        _ => None,
                    })
                    .unwrap_or(0);
                if index < count {
                    if let Some(ClientShellOverlay::PaneMove(pane_move)) = self.overlay.as_mut() {
                        pane_move.selected = index;
                    }
                    self.submit_pane_move(outcome);
                }
            }
            _ => {}
        }
        true
    }
}
