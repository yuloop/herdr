# Verification Report

Date: 2026-07-27

Branch: `feat/window-split-layout`

Validated and deployed commit: `bdd1e291b2fae6524c5fce6731da583cc1143a68`

Upstream state at validation: 22 commits ahead of `upstream/master`, 0 behind. The final merge includes upstream through `dc2506ea`.

## Outcome

The pane transfer, layout rearrangement, restored-CLI navigation, workspace row separation, Qwen integration, and the latest upstream changes passed the applicable local and Linux checks. The Linux musl binary was deployed to `192.168.31.4` through Herdr live handoff. No branch was pushed and nothing was submitted upstream.

## Local validation

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --bin herdr --locked -- -D warnings`: passed.
- Focused post-upstream Rust nextest selection: 85 passed, 0 failed. This includes pane transfer/rearrangement, long-press and context-menu paths, layout presets, host scrolling, handoff repaint eligibility, Qwen, workspace stripes/separators, API schema freshness, manual-copy `Enter/y/Esc` behavior, and the explicit `Ctrl+C/Cmd+C` non-copy regression.
- Python maintenance selection: 77 passed, 0 failed.
- Website tests: 34 passed, 0 failed.
- OpenCode integration asset tests: 4 passed, 0 failed.
- Plugin marketplace tests: 12 passed, 0 failed.
- `openspec validate --all --strict`: 2 passed, 0 failed.
- Agent manifest, configuration reference, English/Japanese/Chinese heading parity, API schema artifact, and 19 historical documentation snapshots: passed.
- `cargo zigbuild --locked --target x86_64-unknown-linux-musl --bin herdr --tests`: passed.
- The cross-compiled Linux test binary was executed on the target host. Post-upstream filters covering Unix handoff repaint, Qwen, host scrolling, pane transfer, manual copy, clipboard handling, delayed agent prompt submission, modify-other-keys Shift+Enter, and last-tab workspace cleanup all passed.

## Scoped exclusions

The repository-wide Windows test command is not a valid all-platform gate in this checkout:

- Integration test targets import `std::os::unix` and `libc::waitpid`, so full `cargo nextest run --locked` does not compile on native Windows.
- The Windows `--bin herdr` run has three unchanged graphics timing tests that time out on this host.
- Five maintenance-script cases exercise Unix checkout/patch semantics and fail under Windows path separators, CRLF, or GBK decoding; the applicable 77-test selection passed.
- The generic integration-asset suite uses Unix sockets; on Windows it reports 5 passed and 8 Unix-socket failures. Linux Rust integration/asset contract tests and the OpenCode asset suite passed; Bun is not installed on the deployment host.

No exclusion covers a changed pane-transfer, layout, scroll, copy, Qwen, or live-handoff behavior.

## Release artifact

- Target: `x86_64-unknown-linux-musl`
- Format: statically linked, stripped x86-64 ELF
- Size: 17,686,576 bytes
- SHA-256: `e72a7257129d763b98c8b5dba6c08ab8a80a1f7d9c56e56a276b651056bba51c`
- Installed path: `/root/.local/bin/herdr`
- Installed and running-process hashes matched the release artifact.

## Backup and handoff

- Fixed single backup: `/root/.local/bin/herdr.backup`
- Backup SHA-256: `7b53ea026cdb82f60c7bbca35645cda7dcc26ac2c24c9f7f83c56ce15be21f3f`
- Old server PID: `3207073`
- New server PID: `1587512`
- Protocol: 18
- Live handoff capability: enabled
- Final status: running, compatible, no restart required

The old server exited naturally after the new server bound the socket. No server or pane process was killed.

## Runtime continuity

All seven original workspace IDs, seven tab IDs, and seven public pane IDs remained present. Shell PIDs remained:

- `w18:p1`: `94137`
- `w1B:p1`: `1632253`
- `w1C:p1`: `1726293`
- `w1D:p1`: `2355528`
- `w1A:p1`: `94325`
- `w1E:p1`: `1138156`
- `w1F:p1`: `1534640`

The active Codex process chain remained `1553561` and `1553614` in `w1A:p1`. The recent terminal content still contained the recorded resume identity for every pane, including the Qwen resume identity in `w1E:p1`. Qwen integration status is `current (v1)`, and the active manifest catalog contains Qwen and Grok.

Live handoff creates new internal terminal wrapper IDs and disconnects the old TUI client processes. A newly launched Herdr client uses the deployed binary and reconnects to the preserved panes. Before a client reconnects, the headless server uses its default 80×24 viewport. Existing handoff serialization carries bounded recent screen history rather than every far-back scrollback row, so deep-history counters are not expected to remain numerically identical; the live processes, public pane IDs, bottom position, current screen/resume content, and recent semantic tail were preserved.

## Live pane-transfer acceptance

An isolated temporary workspace exercised the deployed server:

1. Created two running panes with shell PIDs `1592099` and `1592105`.
2. Repositioned the existing pane with `layout.rearrange`.
3. Applied the rows preset.
4. Detached `w1G:p2` into `w1G:t2`.
5. Joined the same pane back into the split in `w1G:t1`.
6. Confirmed both shell PIDs were unchanged throughout.
7. Closed the temporary workspace and confirmed its panes and processes were gone.

The final production state returned to the original seven workspaces and seven panes.
