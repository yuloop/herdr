# Fork Maintenance Runbook

Use this reference for the custom `deploy/zh-with-perf` branch. Load `.local/herdr-maintenance.local.md` from the repository root for workstation paths and SSH aliases. Never commit that local file or copy credentials into either file.

## Repository topology

- Treat `upstream` as the authoritative `herdrdev/herdr` remote.
- Treat `origin` as the custom fork.
- Keep `master` as a fast-forward-only official mirror.
- Keep custom work on `deploy/zh-with-perf`.
- Do not use optional mirrors as authoritative sync sources. Fetch `upstream` and `origin` independently so a mirror transport failure cannot obscure repository state.

## Fast preflight

Run these before changing source:

```powershell
git status --short --branch
git remote -v
git fetch upstream --prune
git fetch origin --prune
git rev-parse HEAD upstream/master origin/master origin/deploy/zh-with-perf
git log --oneline upstream/master..HEAD
```

Confirm the worktree is clean before `python scripts/sync_upstream.py`. Do not stash user changes to make the script pass.

## Important patch boundaries

Retain these functional lanes until upstream supplies equivalent behavior:

- Managed deploy update isolation: `deploy` builds must not poll or install official application releases; stale official release notes must be discarded. Official stable/preview behavior and agent-detection manifest updates remain active.
- Windows-host Zig cache locality: every Zig target built on Windows must keep global and local caches on the checkout drive. This avoids Zig 0.15.2 cross-drive assertions during Linux musl Clippy.

Find their current commits and descendants dynamically instead of depending on historical hashes.

## Windows validation

Read `zig_0_15_2` from the local environment map when available. Otherwise discover an installed Zig 0.15.2 binary. Set `ZIG` only for the build process; do not rewrite system `PATH`.

```powershell
$env:ZIG = $zigPath
& $env:ZIG version
.\scripts\windows_check.ps1 -Mode check
```

Require Zig `0.15.2`. The complete Windows gate must cover formatting, native Windows Clippy with warnings denied, Linux musl Clippy with warnings denied, maintenance checks, Windows Terminal profile tests, Rust/IPC/native-input/UI tests, and the final Windows build.

For update-policy edits, also run focused tests proving:

- `stable` and `preview` official update behavior remains enabled;
- `deploy` and other externally managed channels are disabled;
- stale official release notes are discarded;
- background application-update scheduling is suppressed for deploy builds;
- agent-detection manifest updates remain independent.

## Formal dual-platform build and deployment

Use the `Sync upstream, build, and deploy` workflow on `deploy/zh-with-perf`. Watch every job to completion. The workflow must build Linux and Windows from the same candidate SHA and must not promote or deploy when either platform fails.

Typical GitHub CLI flow, after confirming authentication and deployment authorization:

```powershell
gh workflow run sync-build-deploy.yml --ref deploy/zh-with-perf
gh run list --workflow sync-build-deploy.yml --limit 5
gh run watch <run-id> --exit-status
```

Download the Windows artifact from the successful run into the ignored `release/` directory. Match its name to the candidate SHA; never use an older artifact merely because its filename looks similar.

Validate `BUILD_INFO.txt` and the ZIP before extraction:

- source SHA equals the promoted custom branch;
- target is `x86_64-pc-windows-msvc`;
- Rust and Zig versions match the workflow contract;
- optimization and SIMD match the contract;
- language is verified as `zh`;
- version and protocol match the binary;
- the recorded ZIP SHA-256 equals the downloaded file;
- every archive entry remains under the staging root;
- the package contains the executable, profile installer, icon, ConPTY metadata/DLL, both host architectures, and notices.

## Safe Windows deployment

Deploy only after an explicit request. Read `windows_install`, `windows_config`, `windows_release_notes`, and `windows_default_cwd` from the local environment map and resolve them before use.

1. Stop Herdr gracefully and verify no `herdr.exe` process remains before overwriting files.
2. Resolve staging and installation paths to absolute paths and require the destination to equal `windows_install` exactly.
3. Record the configuration SHA-256.
4. Copy each staged file by validated relative path. Reject rooted paths and `..` traversal. Do not recursively delete the installation tree and do not create a backup.
5. Compare every installed file hash with staging and confirm the configuration hash is unchanged.
6. Run the packaged profile installer from `windows_install` with `windows_default_cwd`, `-SetDefault`, and `-Elevate`.
7. Validate the installed executable with `--version`, `status client --json`, and `update`.

The update command must exit `1` and contain:

```text
self-update is disabled for deploy builds; install updates through the distribution deployment workflow
```

8. Start the server from the installed executable with a hidden background window, poll `status server --json`, and require the expected version, protocol, compatibility, socket, and executable path.
9. Confirm `windows_release_notes` is absent. Search only log entries after the new UTC startup time and require zero `update.check` and `update.available` entries.
10. Reopen the Windows client. An already rendered settings page can retain old in-memory state until reopened.

## Linux acceptance

The workflow owns Linux replacement and live handoff. Do not manually copy over the binary during a normal release.

For direct verification, load `ssh-ftp-skill` and use its Python helpers. Read `linux_deploy_alias` and `linux_fallback_alias` from the local environment map. Start with the deployment alias; if that transport times out without output, retry the read-only audit through the fallback alias. Never use raw SSH.

Read `linux_binary`, `linux_source_state`, and `linux_release_notes` from the local environment map. Require all of the following:

- the installed binary version matches the candidate;
- client and server status report the same version and protocol;
- server status is running, compatible, live-handoff capable, and detached-daemon capable;
- the source-state file contains the full candidate SHA;
- `herdr update` exits `1` with the managed-build message;
- the release-note cache is absent before and after that check;
- the last official update event predates the new server handoff;
- the running process resolves to the configured Linux binary.

## Final repository check

Fetch `upstream` and `origin` separately once more. Require:

- `HEAD` equals `origin/deploy/zh-with-perf` after an authorized push;
- `upstream/master` equals `origin/master`;
- `upstream/master` is an ancestor of the custom branch;
- the worktree is clean except for explicitly retained, reported changes.

Report exact SHAs and versions. Mention transport or external manifest warnings separately from application-update failures.
