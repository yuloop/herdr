## ADDED Requirements

### Requirement: Qwen Code agent recognition
Herdr SHALL recognize Qwen Code as the canonical `qwen` agent and SHALL use `qwen` as its interactive executable.

#### Scenario: Direct Qwen executable
- **WHEN** a pane foreground process is named `qwen`
- **THEN** Herdr identifies the pane agent as `qwen`

#### Scenario: Wrapped Qwen Code launcher
- **WHEN** a supported runtime wrapper launches a `qwen` or `qwen-code` entrypoint
- **THEN** Herdr identifies the foreground job as the canonical `qwen` agent

#### Scenario: Start Qwen agent by kind
- **WHEN** a user starts an agent with kind `qwen`
- **THEN** Herdr launches the canonical `qwen` executable

### Requirement: Qwen Code screen fallback
Herdr SHALL bundle a Qwen Code detection manifest that classifies only visible live-state evidence and SHALL use known-agent idle fallback when no rule matches.

#### Scenario: Visible approval prompt
- **WHEN** a recognized Qwen pane visibly presents an approval or permission choice
- **THEN** screen detection reports the agent as blocked

#### Scenario: Visible active progress
- **WHEN** a recognized Qwen pane visibly presents its escape-to-cancel active-turn hint
- **THEN** screen detection reports the agent as working

#### Scenario: No live evidence
- **WHEN** a recognized Qwen pane contains no matching live-state rule
- **THEN** screen detection uses the known-agent idle fallback

### Requirement: Qwen integration discovery and control
Herdr SHALL expose `qwen` as an additive integration target through its serialized API, CLI install/uninstall commands, status listing, and settings recommendations.

#### Scenario: Qwen executable is available
- **WHEN** `qwen` is executable on the current PATH
- **THEN** the Qwen integration recommendation is marked available

#### Scenario: Qwen target is serialized
- **WHEN** an integration API request or response contains the Qwen target
- **THEN** it serializes as `qwen`

#### Scenario: Integration status is listed
- **WHEN** the user runs `herdr integration status`
- **THEN** the output includes the Qwen integration and its resolved hook path

### Requirement: Non-destructive Qwen hook installation
The Qwen integration installer SHALL resolve `$QWEN_HOME` or default to `~/.qwen`, SHALL install a versioned Herdr-owned hook, and SHALL merge exact Herdr hook entries into the global Qwen `settings.json` without changing unrelated settings or hooks.

#### Scenario: Install beside existing settings
- **WHEN** Qwen settings contain authentication, model, permission, MCP, or third-party hook entries
- **THEN** installation preserves those values and adds the Herdr entries

#### Scenario: Reinstall current integration
- **WHEN** the Qwen integration is installed more than once
- **THEN** every expected event contains exactly one matching Herdr hook command

#### Scenario: Invalid settings file
- **WHEN** the Qwen settings file is malformed JSON or its `hooks` value is not an object
- **THEN** installation fails with the settings path and does not replace user configuration

#### Scenario: Qwen config directory is absent
- **WHEN** the resolved Qwen configuration directory does not exist
- **THEN** installation fails with guidance to install Qwen Code first

### Requirement: Qwen session and lifecycle reporting
The installed Qwen hook SHALL report Qwen session identity and authoritative lifecycle state only for Herdr-managed panes, and SHALL remain silent and non-blocking to Qwen Code.

#### Scenario: Session starts or resumes
- **WHEN** Qwen emits `SessionStart` with a non-empty session identifier and source
- **THEN** the hook registers that session for the current Herdr pane and reports the pane idle

#### Scenario: Qwen begins work
- **WHEN** Qwen accepts a user prompt, runs or completes a tool, or compacts active context
- **THEN** the hook reports the pane working

#### Scenario: Qwen requests permission
- **WHEN** Qwen emits `PermissionRequest` or a `permission_prompt` notification
- **THEN** the hook reports the pane blocked

#### Scenario: Qwen turn ends
- **WHEN** Qwen emits `Stop`, `StopFailure`, or an `idle_prompt` notification
- **THEN** the hook reports the pane idle

#### Scenario: Qwen session ends
- **WHEN** Qwen emits `SessionEnd`
- **THEN** the hook releases Qwen integration ownership for the pane

#### Scenario: Hook runs outside a Herdr pane
- **WHEN** required Herdr pane environment variables are absent
- **THEN** the hook exits successfully without sending a report or writing output

### Requirement: Safe Qwen integration removal and health status
Herdr SHALL remove only its own Qwen hook commands and hook file, and SHALL report an installed hook as current only when its version and expected settings registrations are valid.

#### Scenario: Uninstall with third-party hooks
- **WHEN** a user uninstalls Qwen integration from settings that also contain third-party hooks
- **THEN** Herdr removes its own commands and file while preserving all third-party entries

#### Scenario: Registration is missing
- **WHEN** the versioned Qwen hook file exists but any expected Herdr settings registration is missing or invalid
- **THEN** integration status reports the Qwen integration as outdated

#### Scenario: Reinstall repairs registration
- **WHEN** the user reinstalls an outdated Qwen integration
- **THEN** Herdr restores all expected registrations and status becomes current

### Requirement: Qwen native session restore
Herdr SHALL preserve a valid Qwen session id reported by the official integration and SHALL resume that exact conversation after a cold session restore when native agent restore is enabled.

#### Scenario: Restore an integration-reported Qwen session
- **WHEN** a saved pane has an id session reference from `herdr:qwen` for the `qwen` agent
- **THEN** Herdr starts the pane with `qwen --resume <id>`

#### Scenario: Reject an untrusted Qwen session reference
- **WHEN** a Qwen session reference comes from a non-official source, has a path reference, or contains an invalid id
- **THEN** Herdr does not construct a native Qwen resume plan

#### Scenario: Restore with the existing Qwen hook version
- **WHEN** the installed version 1 Qwen hook has reported a valid session id
- **THEN** native restore uses that id without requiring the hook to be reinstalled
