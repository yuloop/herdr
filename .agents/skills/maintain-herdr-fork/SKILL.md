---
name: maintain-herdr-fork
description: Maintain the customized Herdr fork across upstream synchronization, local patch isolation, Windows/Linux parity, validation, packaging, deployment, and post-deploy checks. Use when asked to sync the official repository, organize or reapply local patches, maintain the Windows and Linux variants together, build/package/deploy Herdr, investigate the customized Windows build showing an official update, or verify the fork after upstream changes. Triggers include 同步官方仓库, 整理本地补丁, Win/Linux 同步修改, 编译, 打包, 部署, and 设置提示更新.
---

# Maintain Herdr Fork

Use this skill only inside the customized Herdr fork. Preserve upstream history, keep fork behavior reviewable, and require evidence from both operating systems before declaring a deployment complete.

## Load the sources of truth

1. Read the repository-root `AGENTS.md` completely.
2. Read `.github/FORK_AUTOMATION.md` completely. Treat its patch lanes, pitfall ledger, automation contract, and deployment invariants as authoritative.
3. Read `references/runbook.md` for commands, artifact checks, and post-deployment acceptance criteria.
4. Read the repository-root `.local/herdr-maintenance.local.md` when it exists. Treat it as the machine-local environment map, never commit it, and discover current values when it is absent or stale.
5. For build, package, or deployment work, also follow the available `auto-build-deploy` skill.
6. For direct Linux host access, also follow the available `ssh-ftp-skill`; never bypass it with raw `ssh` or `scp`.

Derive current SHAs, versions, artifact names, workflow run IDs, and process state at execution time. Never reuse values from a previous report merely because they appear in the runbook or conversation.

## Respect the requested scope

- For audit, explanation, diagnosis, or status requests, remain read-only apart from harmless fetches and diagnostics. Do not sync, push, deploy, stop services, or clear state unless the user also requests the change.
- For implementation requests, edit and validate the requested patch but do not infer deployment permission.
- For explicit sync requests, run the repository sync flow only after the patch stack and worktree satisfy its preconditions.
- For explicit build/deploy requests, continue through post-deployment runtime verification. A green build alone is not deployment completion.

## Follow the maintenance workflow

### 1. Establish the baseline

- Inspect `git status`, the current branch, all remotes, `upstream/master`, `origin/master`, and the custom branch.
- Preserve unrelated user changes. Stop if they overlap the requested patch and cannot be separated safely.
- Fetch `upstream` and `origin` independently so an optional mirror failure cannot obscure authoritative remote state.
- Inventory the installed Windows and Linux versions and server state before replacement.

### 2. Keep patch lanes separate

- Inspect `git log --oneline upstream/master..HEAD` and classify each local commit using `.github/FORK_AUTOMATION.md`.
- Keep upstream merge commits separate from fork behavior.
- Keep shared behavior, Windows runtime, Linux deployment, packaging, localization, and unrelated features in distinct commits where practical.
- Preserve the managed-deploy update policy unless official upstream now provides an equivalent policy and tests.
- Preserve the Windows-host Zig cache fix unless the vendored build no longer invokes Zig across drives or upstream contains an equivalent fix.
- When a Windows bug can occur in shared code, implement and test the shared fix; do not hide it behind a Windows-only branch.

### 3. Synchronize official upstream

- Require a clean worktree with pending local lanes committed.
- Use `python scripts/sync_upstream.py`; do not replace it with an ad hoc rebase or squash.
- Resolve semantic conflicts by preserving both upstream behavior and intentional fork behavior.
- Let the script run its host checks and create the standard merge commit. Do not push or deploy if it aborts.
- After synchronization, verify that the custom candidate contains the current `upstream/master` and that `master` remains the fast-forward-only official mirror.

### 4. Implement cross-platform changes

- Identify whether the behavior is shared or truly host-specific before editing.
- Add a platform-neutral regression test for shared behavior. Add paired native tests when host APIs intentionally differ.
- Check the counterpart platform for the same assumption whenever either Windows or Linux exposes a bug.
- Keep official self-update enabled for official `stable` and `preview` builds, disabled for externally managed `deploy` builds, and independent from agent-detection manifest updates.

### 5. Validate proportionally, then fully

- Run focused tests while iterating.
- Run formatting, warnings-as-errors Clippy, maintenance tests, UI tests, IPC/protocol tests, and package tests required by the touched lane.
- On Windows, include both the native Windows gate and Linux musl cross-target Clippy. Pin Zig 0.15.2 and keep Zig caches on the checkout drive.
- Require native Windows and native Linux CI for the same candidate SHA before promotion.
- Require packaged artifacts to reject `herdr update` with the managed-build message while official-channel regression tests still pass.

### 6. Deploy only validated artifacts

- Deploy Linux through `.github/workflows/sync-build-deploy.yml` so live handoff, workspace/pane preservation, rollback, and source-state recording remain active.
- Deploy Windows from the Windows artifact produced for that same candidate SHA. Verify `BUILD_INFO.txt`, ZIP SHA-256, safe archive paths, required package files, language, protocol, and version before copying.
- Preserve the Windows configuration, use the exact configured install directory, and do not create a backup. Replace only validated runtime/package files.
- Reinstall the packaged Windows Terminal profile with the configured starting directory, default-profile setting, and elevation setting.

### 7. Prove the runtime result

- Verify installed client and running server versions, protocol compatibility, source SHA, executable path, and process command line.
- Verify manual self-update exits with the expected managed-deploy rejection on both platforms.
- Verify cached official release notes are absent and no `update.check` or `update.available` event occurs after the new server starts or hands off.
- Compare log timestamps in UTC when the host reports local deployment times.
- Reopen or restart the Windows client after replacement so a previously rendered settings screen cannot retain stale UI state.
- Treat an SSH transport timeout separately from a deployment failure; retry through the configured alternate alias before drawing a runtime conclusion.

## Report completion with evidence

State the official upstream SHA, custom SHA, Windows and Linux versions, protocol, CI/workflow result, installation targets, update-policy result, cache state, and worktree state. Disclose any non-blocking warning separately. Never say "fixed" or "deployed" when either platform lacks runtime evidence.
