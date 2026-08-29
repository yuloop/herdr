## Why

Herdr cannot currently recognize Qwen Code sessions or install lifecycle reporting for them, so Qwen Code panes appear as generic terminals and do not participate reliably in agent status workflows. Qwen Code now exposes stable command hooks and session identifiers, making a first-class integration possible without replacing users' existing hook configuration.

## What Changes

- Add Qwen Code (`qwen`) to process identification and screen-manifest detection.
- Add a `qwen` integration target to the API, CLI, settings recommendations, and integration status reporting.
- Install a Herdr-owned Qwen Code hook script and merge only Herdr-owned hook entries into `~/.qwen/settings.json` (or `$QWEN_HOME/settings.json`).
- Report Qwen Code session identity and lifecycle states (`working`, `blocked`, `idle`, and release) from official hook events.
- Resume saved Qwen Code conversations with the integration-reported session id after a cold Herdr restore.
- Make uninstall remove only Herdr-owned files and hook entries, preserving unrelated Qwen Code settings and hooks.
- Add tests, bundled detection data, and user documentation for detection, installation, status, and cleanup.

## Capabilities

### New Capabilities

- `qwen-code-agent-integration`: First-class detection and optional lifecycle integration for Qwen Code sessions.

### Modified Capabilities

None.

## Impact

This affects agent detection manifests, integration target schemas, integration configuration editing and assets, CLI/API serialization, native agent resume planning, settings recommendations, generated API documentation, and integration/detection tests. It introduces no new runtime dependency and no breaking change; `qwen` is an additive integration target.
