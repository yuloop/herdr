## 1. Characterization and transfer core

- [x] 1.1 Add failing characterization tests for title-only drag activation, transfer cancellation, target resolution, and reuse of existing pane move semantics
- [x] 1.2 Add the stable pane-transfer interaction state and an App runtime mutation wrapper that commits through existing `pane.move` behavior

## 2. Pane transfer interaction

- [x] 2.1 Implement long-press pane-title dragging, pane-edge/tab/new-tab/new-workspace drop targets, pure preview rendering, and invalid-drop cancellation
- [x] 2.2 Add the right-click move-or-detach fallback, keyboard navigation, localized labels, error toast handling, and cross-workspace regression coverage

## 3. Restored CLI navigation

- [x] 3.1 Add per-pane handoff repaint eligibility, reset replayed panes to live output, and skip redundant resize nudges when a usable screen was restored
- [x] 3.2 Implement page-sized and endpoint modifier scrolling for full-app and direct-attach host scrollback while preserving application-owned wheel routing

## 4. Validation and deployment

- [x] 4.1 Run targeted tests, formatting/lint checks, the repository validation recipe, and a Linux release build; record any scoped exclusions
- [x] 4.2 Back up and live-handoff deploy the validated binary to `192.168.31.4`, then verify pane IDs, child PIDs, agent sessions, layout moves, and scroll continuity without submitting upstream
