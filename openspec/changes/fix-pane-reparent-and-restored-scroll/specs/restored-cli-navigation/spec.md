## ADDED Requirements

### Requirement: Restored CLI panes open at live output
The system SHALL present the newest available output after importing a live pane and SHALL not require manual scrolling to reach the live view.

#### Scenario: Import a primary-screen pane with replay history
- **WHEN** a live handoff imports a primary-screen pane with usable history ANSI
- **THEN** the imported emulator is positioned at offset zero from the bottom before the first client frame

#### Scenario: Import a pane without replay history
- **WHEN** a live handoff imports a pane without usable history ANSI
- **THEN** the system requests an application repaint so the first attached client is not left with a blank pane

### Requirement: Handoff avoids redundant full CLI redraws
The system SHALL avoid the shrink-and-restore repaint nudge for imported panes that already contain a usable replayed screen.

#### Scenario: First client attaches to a replayed main-screen CLI
- **WHEN** the first client attaches after handoff and the imported pane already has replayed history
- **THEN** the pane is resized directly to the client's final geometry without an additional transient resize nudge

#### Scenario: First client geometry differs
- **WHEN** the attached client's final pane geometry differs from the imported PTY geometry
- **THEN** the system performs the required final resize once and keeps the viewport at live output

#### Scenario: Pane needs application repaint
- **WHEN** the imported pane has no usable replayed screen
- **THEN** the existing repaint nudge remains available for that pane only

### Requirement: Long history supports accelerated mouse navigation
The system SHALL support normal, page-sized, and endpoint mouse scrolling while Herdr owns host scrollback.

#### Scenario: Plain wheel uses configured step
- **WHEN** the user scrolls without a modifier in host scrollback
- **THEN** the pane moves by the configured `ui.mouse_scroll_lines` amount

#### Scenario: Shift wheel scrolls a page
- **WHEN** the user holds Shift and scrolls in host scrollback
- **THEN** the pane moves by approximately one visible viewport

#### Scenario: Control or reported Command wheel jumps to an endpoint
- **WHEN** the user holds Control, or the host terminal reports Command as Super or Meta, and scrolls upward
- **THEN** the pane jumps to the oldest retained position
- **WHEN** the user holds Control, or the host terminal reports Command as Super or Meta, and scrolls downward
- **THEN** the pane returns directly to live output

#### Scenario: Host terminal does not report Command
- **WHEN** the host terminal does not expose Command as a Super or Meta wheel modifier
- **THEN** Control remains available as the universal endpoint-scroll modifier

#### Scenario: Direct attach uses the same acceleration
- **WHEN** the user navigates scrollback through a direct terminal attachment
- **THEN** the modifier semantics match the full Herdr application

### Requirement: Application-owned wheel input remains compatible
The system MUST NOT apply host scrollback acceleration when the pane application owns wheel input.

#### Scenario: Mouse-reporting application
- **WHEN** the active pane requests terminal mouse reporting
- **THEN** wheel events and their modifiers are forwarded to the pane application

#### Scenario: Alternate-scroll application
- **WHEN** the active pane uses alternate-scroll routing
- **THEN** wheel events continue to be encoded for the application rather than navigating host history
