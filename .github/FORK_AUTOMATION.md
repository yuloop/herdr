# Fork sync, build, and deployment automation

The fork's default branch is `deploy/zh-with-perf`. The `master` branch is a
fast-forward-only mirror of `herdrdev/herdr:master`; custom changes never land
on it. The hourly workflow creates a temporary merge candidate. Linux and
native Windows Server 2022 jobs validate that same commit, and the custom
branch is not advanced or deployed until both platforms pass. A candidate is
transferred between jobs as a Git bundle; it is never exposed as a partially
verified remote branch.

The workflow never opens an upstream issue or pull request. Failures are kept
in the fork's single issue named
`[automation] Upstream sync/build/deploy failed`. Repeated identical failures
update that issue in place, and a successful recovery comments and closes it.

## Local patch boundaries

Keep fork work as a reviewable patch stack on `deploy/zh-with-perf`. Do not mix
an upstream merge with new fork behavior, and do not squash unrelated lanes
together. The stable lanes and required validation are:

| Lane | Typical scope | Required validation |
| --- | --- | --- |
| Shared behavior | `src/app/`, `src/config/`, `src/client/`, `src/server/`, shared docs | Native Linux and native Windows checks; prefer one platform-neutral regression test |
| Windows runtime | `src/platform/windows/`, Windows-only IPC and ConPTY adapters | Shared tests plus native Windows tests |
| Linux runtime/deployment | Unix adapters, musl packaging, `scripts/herdr_deploy.py` | Shared tests plus native Linux tests and static artifact checks |
| Platform packaging | Windows profile/installer or Linux artifact scripts | The matching native package test; shared metadata formats must be tested on both |
| Localization | `locales/`, localized docs, translation infrastructure | Translation parity plus both native builds |
| Independent features | Pane, performance, detection, and integrations | Keep one functional concern per commit and apply the shared/platform rule above |

Shared policy belongs in platform-neutral modules. Platform modules should
contain only host API calls and adaptations. When a bug is first observed on
one operating system, check the shared path before adding an OS-specific
workaround. Add a shared regression test when the behavior should match; when
the host APIs genuinely differ, add paired Linux/Windows tests that describe
the same scenario and document the intentional difference.

### Shared pitfall ledger

Keep this table current when a platform fix exposes a shared assumption or an
upstream merge needs semantic conflict resolution.

| Area | Pitfall to preserve |
| --- | --- |
| Mouse selection/settings | Defaults and modal routing are shared behavior. Fix and test them outside `cfg(windows)` even when first reproduced in Windows Terminal. |
| `src/app/runtime.rs`, `src/client/mod.rs` | These are shared hot spots for both terminal appearance/input work and Windows client lifecycle work. An automatic merge is not enough; run both native gates. |
| Windows named pipes | Elevated and non-elevated clients of the same user must interoperate without granting access to other users. Keep the SID-based Windows-only security test. |
| Windows Terminal windows | Restore only the window that owns the current client process. Never enumerate and reposition unrelated Terminal windows. |
| Copy-mode documentation | Upstream may extend copy navigation while the fork changes mouse-copy defaults. Preserve both sets of semantics in English, Japanese, and Chinese docs. |
| Linux-only Clippy lints | A Windows cross-target `cargo check` can miss warnings in `cfg(unix)` code. Keep the Windows-to-Linux musl gate on Clippy with warnings denied, and keep Zig caches on the checkout drive for every target when the build host is Windows. |
| Translation key namespaces | Missing rust-i18n keys compile and render as their dotted key names. Keep `scripts.test_i18n_key_check` in both native gates so every literal `t!` key exists in both `en.yml` and `zh.yml`. |
| Maintenance tests on Windows | Read repository text as UTF-8, compare tracked paths in POSIX form, and keep `*.patch` files LF-only. Run the same Python maintenance suite in both native gates. |
| Shared UI and layout tests | Fix the split ratio and assert the intended viewport height before testing truncation. A localized label must not undo an upstream narrow-screen fallback; measure terminal cell width, not UTF-8 bytes. Keep the shared UI suite in the native Windows gate instead of relying on Unix-only full tests. |
| Packaged PowerShell defaults | Windows PowerShell 5.1 can evaluate `$PSScriptRoot` as empty inside a `param(...)` default expression. Resolve package-relative executable and icon paths in the script body, and test the packaged installer without explicit path overrides. |
| Windows command processor | Terminal launchers can repurpose inherited `ComSpec` to point at themselves. Resolve `cmd.exe` from the Windows system directory and normalize `ComSpec` for pane, plugin, custom-command, and batch-file descendants; never treat arbitrary `ComSpec` as an executable path. |
| Managed deploy updates | Binaries built with `HERDR_BUILD_CHANNEL=deploy` must only be updated by the dual-platform deployment workflow. Keep official background/manual self-update disabled on both platforms and discard cached official release notes so local patches cannot be overwritten. Agent detection manifest updates remain independent and enabled. |

The 2026-08-07 rehearsal against upstream `69a07fdf` had one semantic conflict,
in `docs/next/website/src/content/docs/keyboard.mdx`: keep upstream's big-word
`W/B/E` navigation and the fork's manual drag-selection behavior. All Rust
source changes merged automatically, but the resulting candidate was still
run through the full Windows check rather than being accepted on merge status
alone.

Generated packages belong under the ignored `/release/` directory. They are
validation outputs, never source inputs and never part of an upstream merge.

For a local synchronization, first commit each pending lane and leave the
working tree clean, then run:

```bash
python scripts/sync_upstream.py
```

The tool pins `upstream` to `herdrdev/herdr`, disables pushes to that remote,
creates a timestamped backup branch, performs a non-rebasing `--no-commit`
merge, runs `git diff --check` and the current host's `just check`, and only
then creates the standard upstream merge commit. A local check proves only the
host on which it ran; the `deploy/zh-with-perf` CI and automated promotion gate
remain responsible for native Linux plus native Windows acceptance. A conflict
or failed check aborts the merge; the tool never stashes, rewrites fork
commits, pushes, or deploys. The legacy `scripts/update-from-upstream.sh` and
`scripts/auto-update.py` names are thin compatibility entry points for the same
sync-only flow.

The checks also lint in the opposite direction where practical: Unix
`just check` runs Windows-target Clippy, while Windows `just check` runs Clippy
for the Linux musl target. These checks catch conditional-compilation and
platform-only warning drift early, but they do not replace the native runtime
jobs.

## Repository configuration

Configure these Actions secrets:

- `HERDR_DEPLOY_SSH_KEY`: the private half of the dedicated, passphrase-free
  GitHub Actions deployment key.
- `HERDR_DEPLOY_KNOWN_HOSTS`: a pinned `known_hosts` entry for the exact host
  and port. Do not use `StrictHostKeyChecking=no`.

Configure these Actions variables:

- `HERDR_DEPLOY_HOST=sl.z123j.top`
- `HERDR_DEPLOY_PORT=38887`
- `HERDR_DEPLOY_USER=root`
- `HERDR_DEPLOY_PATH=/root/.local/bin/herdr`

Enable Issues and Actions on the fork, and allow workflows read/write
repository permissions. Set `deploy/zh-with-perf` as the default branch so the
scheduled workflow is loaded from the custom branch.

GitHub Actions failure email is an account notification preference, not a
repository file setting. The account watching this fork must enable workflow
failure notifications in GitHub's notification settings.

## Deployment invariants

The workflow builds `x86_64-unknown-linux-musl` and
`x86_64-pc-windows-msvc` from the same candidate SHA. Both use Rust 1.96.1,
Zig 0.15.2, `ReleaseFast`, and SIMD. The Linux gate verifies static linking,
unresolved C++ runtime symbols, binary version, protocol, and SHA-256. The
custom branch and Linux server remain unchanged until this gate and the native
Windows gate both succeed. Both artifacts must also reject `herdr update` with
the managed-build protection before they can be promoted.

The Windows job runs the repository's native Windows checks, creates a Release
binary, and verifies that `ui.language = "zh"` produces Simplified Chinese CLI
help. It packages the binary with the pinned official ConPTY bundle, verifies
the NuGet hash and signature, verifies Microsoft Authenticode signatures,
probes enhanced ConPTY input, and exercises installation and repair through
Windows PowerShell 5.1. The resulting
`herdr-windows-x86_64-<commit>` Actions artifact contains
`herdr-windows-x86_64.zip` and `BUILD_INFO.txt` with the source commit,
toolchain, version, protocol, and SHA-256. It is retained for 14 days and is
not deployed to the Linux host.

The host-side deployer records workspace IDs and total Pane count before the
handoff. It atomically replaces the installed binary, requests
`server live-handoff`, then requires all of the following:

- server status is `running`;
- the version and protocol match the staged binary;
- the protocol is compatible and live handoff remains available;
- every prior workspace ID is still present;
- total Pane count did not decrease.

The only backup is `/root/.local/bin/herdr.previous`. Successful deployment
writes the full source commit to `/root/.local/bin/herdr.source-sha`. If the
handoff or acceptance checks fail, the deployer restores the previous binary
and performs a reverse live handoff when necessary. The source-state file is
updated only after all acceptance checks pass.

## Manual verification

Run **Sync upstream, build, and deploy** from the Actions tab. A no-change run
should finish before installing build tools when the source-state file already
matches the custom branch. For a full acceptance exercise, verify these cases
in a disposable fork/branch before relying on the hourly schedule:

1. clean upstream merge and deployment;
2. deliberate merge conflict (custom branch and server stay unchanged);
3. failing Linux or Windows test/build (no branch advancement or deployment);
4. unavailable SSH or rejected handoff (old service remains usable);
5. a Windows package build with Chinese `--help`, signed ConPTY files, and a
   passing installer/repair test;
6. two dispatches (the concurrency group serializes them).
