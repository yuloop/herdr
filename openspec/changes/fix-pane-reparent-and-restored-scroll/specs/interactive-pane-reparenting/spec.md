## ADDED Requirements

### Requirement: Existing panes can join another split
The system SHALL allow a running persistent pane to be moved beside a target pane in an existing tab using a selected top, bottom, left, or right placement.

#### Scenario: Drag a pane onto a visible target edge
- **WHEN** the user long-presses a pane title, drags it to an edge of another visible pane, and releases
- **THEN** the source pane is moved into the target tab at that edge and both panes remain running

#### Scenario: Move a pane into another tab
- **WHEN** the user selects another existing tab as the destination
- **THEN** the source pane is moved beside that tab's focused pane using the selected or default placement

#### Scenario: Move a pane across workspaces
- **WHEN** the source and target tabs belong to different workspaces
- **THEN** the pane is reparented using the target workspace's pane identity rules without restarting its PTY

### Requirement: Split panes can detach into independent containers
The system SHALL allow a persistent pane to leave its current split and become the first pane of a new tab or a new workspace.

#### Scenario: Detach into a new tab
- **WHEN** the user drops or confirms the source pane on the new-tab destination
- **THEN** a new tab is created in the selected workspace and owns the same running pane

#### Scenario: Detach into a new workspace
- **WHEN** the user drops or confirms the source pane on the new-workspace destination
- **THEN** a new workspace and initial tab are created around the same running pane

#### Scenario: Detach the only pane in a source container
- **WHEN** moving the source pane leaves its previous tab or workspace empty
- **THEN** the empty source container is removed and focus remains on a valid pane

### Requirement: Pane transfer is available by mouse and menu
The system SHALL expose pane transfer through both title dragging and a pane context-menu workflow.

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

#### Scenario: Keyboard confirmation and cancellation
- **WHEN** the transfer overlay is open
- **THEN** the user can choose a target with the keyboard, confirm with Enter, or cancel with Esc

### Requirement: Pane transfer commits atomically
The system MUST leave the source runtime and layout unchanged until a valid transfer is committed.

#### Scenario: Cancel a transfer
- **WHEN** the user presses Esc or releases over an invalid destination
- **THEN** the overlay closes and the source pane remains in its original tab and layout

#### Scenario: Destination disappears before commit
- **WHEN** the selected target pane, tab, or workspace no longer exists at commit time
- **THEN** the transfer fails visibly and the source pane is restored to its original container

#### Scenario: Runtime continuity
- **WHEN** a pane transfer succeeds
- **THEN** its PTY, child process, cwd, terminal output, and known agent session continue without restart or exit input

### Requirement: Existing same-tab layout tools remain distinct
The system SHALL retain same-tab repositioning and layout templates alongside cross-container pane transfer.

#### Scenario: Use same-tab repositioning
- **WHEN** the user selects the existing reposition action
- **THEN** only the layout tree of the current tab is rearranged and no pane container transfer occurs
