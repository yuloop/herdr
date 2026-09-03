## Context

Herdr currently models 21 detectable agents and 15 installable integrations. Detection is split between foreground-process identity and bundled screen manifests; optional integrations add session identity or lifecycle reports through each agent's supported extension mechanism. Qwen Code's canonical executable is `qwen`, its user settings live in `$QWEN_HOME/settings.json` when `QWEN_HOME` is set and otherwise `~/.qwen/settings.json`, and its official command hooks receive JSON lifecycle payloads on stdin.

The change crosses detection, API schema, CLI parsing, integration configuration editing, platform-specific hook assets, settings recommendations, documentation, and generated schema. Existing Qwen settings and third-party hooks are user data and must not be replaced.

## Goals / Non-Goals

**Goals:**

- Recognize Qwen Code foreground jobs and expose `qwen` as an agent kind with canonical executable `qwen`.
- Provide screen-manifest fallback when the optional integration is absent or stale.
- Expose a cross-platform `qwen` integration target through CLI, API, status, and settings recommendations.
- Use official Qwen Code hooks to report session identity and authoritative lifecycle state.
- Use Qwen Code's session id to resume saved conversations after a cold Herdr restore.
- Make install, reinstall, and uninstall idempotent while preserving unrelated Qwen settings and hooks.

**Non-Goals:**

- Install or update Qwen Code itself.
- Modify project-local `.qwen/settings.json` files.
- Select or resume a Qwen conversation when no integration-reported session id is available.
- Treat Qwen model providers used by other clients as Qwen Code processes.

## Decisions

### Add a distinct `Qwen` agent and bundled manifest

`Agent::Qwen` will use the canonical label and executable `qwen`, with `qwen-code` accepted as an identification alias for wrapped launchers. It will be included in agent enumeration, screen-manifest agents, bundled manifests, website detection data, CLI kind documentation, and validation tests.

The initial manifest will use conservative visible evidence: Qwen/Gemini-style approval prompts are blocked and an explicit escape-to-cancel progress hint is working. Unmatched known Qwen panes retain the engine's known-agent idle fallback. This avoids classifying transcript text as live state.

Alternative considered: process identity only. Rejected because installations without hooks would never distinguish visible approval waits or active turns.

### Merge Herdr-owned entries into the global Qwen settings

The installer will resolve `QWEN_HOME`, falling back to `~/.qwen`, require that directory to exist as proof Qwen Code is installed, write a platform hook under `hooks/`, and parse/update only the top-level `hooks` object in `settings.json`. Existing helper functions for nested command-hook groups will add or remove exact Herdr command entries without normalizing or deleting foreign entries.

Alternative considered: own a separate complete settings file. Rejected because Qwen Code reads a shared settings document and replacing it could destroy authentication, model, permission, MCP, and third-party hook settings.

### Report full lifecycle state through official hook events

The Herdr hook will accept an explicit action and read `session_id`, `source`, and `hook_event_name` from stdin. The install maps events as follows:

- `SessionStart` → register session identity using the provided start source and report `idle`
- `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PreCompact`, and `PostCompact` → `working`
- `PermissionRequest` and `Notification` matched to `permission_prompt` → `blocked`
- `Notification` matched to `idle_prompt`, `Stop`, and `StopFailure` → `idle`
- `SessionEnd` → release integration ownership

The source/agent pair `herdr:qwen` / `qwen` will be marked as full-lifecycle authority. Scripts are best-effort, silent, bounded, and no-op outside Herdr-managed panes. Unix sends socket requests directly with Python 3; Windows uses the Herdr CLI.

Alternative considered: session identity only with screen-derived lifecycle. Rejected for the first implementation because Qwen exposes direct permission, stop, failure, and session-end events that are more precise than terminal text. Screen detection remains a fallback and a visible-blocker safety signal.

### Resume the exact integration-reported Qwen session

Qwen Code exposes `qwen --resume <sessionId>` for non-interactive selection of a saved conversation. Herdr will accept id references only from the exact official source/agent pair `herdr:qwen` / `qwen` and will build the resume argv as separate values so the session id is never interpreted as shell syntax.

The existing version 1 hook already reports Qwen's stable `session_id`, so enabling native restore does not require an integration asset change or reinstall. `qwen --continue` was considered but rejected because it selects the most recent project session rather than the exact session bound to the restored pane.

### Treat hook-file and hook-registration health as one integration

Status will use the version marker in the installed hook file and also validate that all expected Herdr hook registrations are present in Qwen settings. A current script with missing configuration is reported as outdated so reinstall can repair it.

Alternative considered: check the script version only. Rejected because Qwen will never invoke an unregistered script.

## Risks / Trade-offs

- [Qwen changes hook event names or payloads] → Keep parsing defensive, ignore malformed input, version the integration asset, and cover the documented payload contract with tests.
- [A hook event arrives out of order] → Use monotonic report sequence values so Herdr's existing stale-report arbitration rejects older state.
- [User settings contain malformed JSON or a non-object `hooks` value] → Fail without writing either configuration or a partial replacement; report the exact file.
- [Conservative screen patterns miss a new Qwen UI state] → Hooks remain authoritative when installed, and the versioned remote-manifest mechanism can update screen rules independently.
- [Hook subprocess overhead] → Commands are asynchronous from Herdr's perspective, use short timeouts, and perform only a local socket/CLI call.

## Migration Plan

1. Ship the additive agent enum, manifest, API enum variant, hook assets, and integration plumbing in one binary.
2. Existing users opt in with `herdr integration install qwen`; no existing configuration is migrated automatically.
3. Existing version 1 Qwen integrations begin supporting native restore after the Herdr binary update without reinstalling the hook.
4. Reinstall repairs Herdr-owned entries and leaves other Qwen configuration intact.
5. Rollback with `herdr integration uninstall qwen`, which removes only Herdr-owned entries and the hook file.
6. An older Herdr binary can continue running after the new binary is placed on disk; activate the deployed server through live handoff where supported so pane processes remain alive.

## Open Questions

None for this scope.
