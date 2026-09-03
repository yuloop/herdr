## MODIFIED Requirements

### Requirement: Pane transfer is available by mouse and menu
The system SHALL expose pane transfer through both title dragging and a pane context-menu workflow, and the context-menu destination picker SHALL keep detach destinations directly selectable and identify move destinations with user-facing context plus a stable pane identity.

#### Scenario: Title dragging starts only from the title region
- **WHEN** the user presses and drags inside terminal content rather than the rendered pane title region
- **THEN** pane transfer does not start and existing selection or pane mouse routing continues

#### Scenario: Drag within the source tab uses layout rearrangement
- **WHEN** the user drags a pane title to an edge of another pane in the same tab
- **THEN** the existing same-tab repositioning semantics arrange the panes without performing a container transfer

#### Scenario: Pane has no visible title handle
- **WHEN** pane borders are hidden, the title is empty, or the pane is too narrow to render a title
- **THEN** title dragging is unavailable for that pane and the context-menu transfer workflow remains available

#### Scenario: Context-menu fallback
- **WHEN** the user chooses the move-or-detach action from a pane context menu
- **THEN** the same destination and placement model used by title dragging is presented

#### Scenario: Detach remains visible with many split panes
- **WHEN** the destination count exceeds the transfer picker's visible row limit
- **THEN** the new-tab and new-workspace destinations remain in the directly visible and clickable rows

#### Scenario: Move destinations are identifiable
- **WHEN** multiple destination panes exist, including panes with the same title
- **THEN** each move row shows its workspace, tab, pane title when available, and stable pane identity so the user can distinguish the target

#### Scenario: Keyboard confirmation and cancellation
- **WHEN** the transfer overlay is open
- **THEN** the user can choose a target with the keyboard, confirm with Enter, or cancel with Esc
