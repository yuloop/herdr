# REBASE_INVENTORY（只统计、不改代码）

> 结论：fork 独有 135 个提交（106 非合并 + 29 合并）、201 个文件。上游只前进了 23 个提交，但重构极大（MB..HEAD 约 197 文件、5.6万加/6.2万删），真正的搬运难点是 `src/app/input/*`、`src/ui/*`、`src/app/runtime_mutations.rs` 已被上游删除，以及 `website/*` 已搬家到 `distribution/*` 或独立仓库。

- 基线 MB：`5158adab fix(windows): launch powershell agent shims with arguments (#3455)`
- 对比目标：`origin/main ac37d8d1 feat: graceful degradation for Windows nightly build`
- 当前分支：`feat/herdr-thorough-rebase`，HEAD `94f6d9c0 fix: reveal newly focused spaces in the sidebar (#3554)` = `upstream/master`，干净。
- 统计命令：
  - `git rev-list --count 5158adab..origin/main` = 135（`--merges` 29，`--no-merges` 106）
  - `git diff --name-only 5158adab..origin/main | wc -l` = 201
  - `git rev-list --count 5158adab..upstream/master` = 23
- 难度定义：
  - 易：纯新增且上游未动，直接拷贝即可，只需过编译。
  - 中：路径不变但上游小改（一般 <100 行），或纯改名可机械迁移。
  - 难：文件被上游删除/拆分，或上游大改（数百至上万行），需在新架构上重做。

---

## 1) 按功能分组（提交 hash + 标题）

### A. 中文汉化 i18n（核心，重复提交因分支 reconcile 出现两遍）
功能一句话：引入 rust-i18n + locales 中英文资源 + 默认中文 + 全量 UI/CLI/设置汉化。

- `5aa82767` i18n: 简体中文汉化
- `e6b3e4b3` i18n: 简体中文汉化（重演）
- `6d153bb3` fix(i18n): CLI 子命令分发前应用 locale,修复 update/status 输出英文
- `5c3c0733` fix(i18n): CLI 子命令分发前应用 locale（重演）
- `11b692d1` fix(i18n): 修复设置面板中文标签点击区域错位
- `37b90fdc` fix(i18n): 修复设置面板中文标签点击区域错位（重演）
- `e4c733d1` fix(i18n): localize settings and integration status
- `c17ebc8c` fix(i18n): preserve diagnostics on narrow screens
- `fa734549` fix(i18n): validate translation keys across platforms
- `b0bd2d7b` feat(i18n): change default language to Chinese (zh)
- 核心文件：`src/i18n.rs(新增)`、`locales/en.yml(新增)`、`locales/zh.yml(新增)`、`src/config.rs`、`src/config/model.rs`、`src/cli/spec.rs`、`src/main.rs`、`src/ui/settings.rs`、`src/app/input/settings.rs`、`src/app/input/sidebar.rs`、`scripts/i18n_key_check.py(新增)`、`I18N.md(新增)`、`docs/next/**/zh-cn/*.mdx`、`README.zh-CN.md`

### B. Qwen agent 集成 + 检测
功能一句话：新增 qwen 集成（会话继承、状态脚本、注册表）+ 检测 npm/独立启动器 + grok 背景工作修复。

- `78269b41` feat: add qwen code agent integration
- `e27b6a3c` feat: add qwen code agent integration（重演）
- `8a77b50a` feat: support qwen code session inheritance
- `f2bd97f4` feat: support qwen code session inheritance（重演）
- `a0bf5795` fix: detect current qwen npm launchers
- `781d2b8d` fix: detect current qwen npm launchers（重演）
- `3e9bad3a` fix(detect): recognize standalone qwen launcher
- `f32c07a1` fix(detect): recognize standalone qwen launcher（重演）
- `4902001a` fix: detect grok background work
- `52d28433` fix: detect grok background work（重演）
- 核心文件：`src/detect/manifests/qwen.toml`、`src/detect/manifest.rs`、`src/detect/mod.rs`、`src/integration/*`、`src/integration/assets/qwen/*(新增)`、`src/cli/integration.rs`、`src/agent_resume.rs`、`website/agent-detection/qwen.toml`、`website/agent-detection/grok.toml`、`website/agent-detection/index.toml`、`scripts/agent_detection_manifest_check.py`、`openspec/changes/add-qwen-code-agent-integration/*(新增)`、`openspec/changes/fix-qwen-current-npm-launcher-detection/*(新增)`、`docs/superpowers/reports/2026-07-27-fix-qwen-current-npm-launcher-detection-verify.md(新增)`

### C. Pane 布局/转移/滚动/工作区
功能一句话：pane 跨工作区转移、预览、detach 选目标、滚动加速、workspace 条目拆分、origin 保留。

- `ecaf9172` fix: make pane moves exactly reversible
- `3e38152c` fix: make pane moves exactly reversible（重演）
- `02c70d4e` feat: add stable pane transfer previews
- `25ae300e` feat: add stable pane transfer previews（重演）
- `c7a11699` feat: add interactive pane transfer controls
- `93e11cdd` feat: add interactive pane transfer controls（重演）
- `b6085a34` feat: commit pane transfers through runtime mutations
- `dc476da9` feat: commit pane transfers through runtime mutations（重演）
- `6cb03fb6` fix: avoid redundant handoff repaint nudges
- `50b00106` fix: avoid redundant handoff repaint nudges（重演）
- `b6ce8e7b` feat: accelerate terminal history scrolling
- `6cc95dab` feat: accelerate terminal history scrolling（重演）
- `028f7597` feat: add pane layout rearrangement API
- `8333fcad` feat: add pane layout rearrangement API（重演）
- `93099690` feat(panes): preserve origins across workspace moves
- `e08ae154` feat(panes): preserve origins across workspace moves（重演）
- `074fb8ac` fix: improve pane detach target selection
- `f4cc7c97` fix: improve pane detach target selection（重演）
- `034b750a` feat: separate workspace list entries
- `d86d0af2` feat: separate workspace list entries（重演）
- `49efbc36` feat(client): adapt host cursor rendering automatically
- `20911b7a` feat(client): adapt host cursor rendering automatically（重演）
- 核心文件：`src/app/input/pane_layout.rs(新增)`、`src/ui/pane_layout.rs(新增)`、`src/app/runtime_mutations.rs`、`src/app/runtime.rs`、`src/app/state.rs`、`src/app/mod.rs`、`src/app/api/panes.rs`、`src/app/api/layouts.rs`、`src/workspace.rs`、`src/workspace/tab.rs`、`src/pane.rs`、`src/pane/terminal.rs`、`src/layout.rs`、`src/api/schema/panes.rs`、`openspec/changes/fix-pane-reparent-and-restored-scroll/*(新增)`、`openspec/changes/fix-pane-transfer-detach-picker/*(新增)`

### D. Windows 打包/终端/IPC/构建
功能一句话：Windows x64 包、终端 profile 安装器、同用户提权 IPC、默认 shell 选择、ConPTY、Zig 缓存。

- `4bca89e8` feat(windows): localize and automate x64 package
- `86c41be9` fix(windows): allow same-user ipc across elevation
- `74d06d19` fix(windows): restore client window state and harden launch
- `87bc8ce4` feat(windows): package terminal profile installer
- `67392def` fix(windows): choose an available default shell
- `dea5e99b` fix(windows): resolve packaged terminal profile defaults
- `d5acecf0` fix(windows): use the system command processor for batch files
- `0543efe5` fix(build): keep Zig caches on the Windows source drive
- `537fe84d` fix(update): protect managed deploy builds from official updates
- 核心文件：`src/platform/windows.rs`、`src/platform/windows/client_window.rs(新增)`、`src/ipc.rs`、`src/ipc/windows_listener.rs(新增)`、`src/client/input/windows_vti.rs`、`src/client/mod.rs`、`src/update.rs`、`build.rs`、`Cargo.toml`、`scripts/package_windows_conpty.py`、`scripts/install_windows_terminal_profile.ps1(新增)`、`scripts/windows_terminal_profile_test.ps1(新增)`、`scripts/windows_check.ps1`

### E. PTY 性能批量读取
功能一句话：unix PTY 一次唤醒批量读 64 次/512KB，用 dirty 标志合并渲染，高速输出不卡。

- `17f999e8` perf(pty): 批量读取 PTY 输出,减少高速输出时的渲染唤醒
- `b70f4de7` perf(pty): 批量读取 PTY 输出（重演）
- `d80bdb0a` feat: integrate localization, manual copy, and pty batching（集成提交）
- 核心文件：`src/pty/actor/unix.rs`（上游已大改 607 行，难）

### F. Fork 自动化 CI（sync/build/deploy）
功能一句话： hourly 同步上游、每日两次部署、CN 同步/发布/夜版三件套、防 workflow 推送拒绝。

- `f2509d9a` ci: automate fork sync, build, and deployment
- `e98c23e9` ci: automate fork sync, build, and deployment（重演）
- `7234d976` chore(repo): add safe upstream sync workflow
- `83f5e8d8` ci: gate fork promotion on linux and windows
- `4757351b` ci: add herdr-cn pipeline (sync/release/nightly) + CN docs on stable base
- `bfd59d19` chore: add fork maintenance skill
- `71951df4` ci: deploy only twice daily at 08:00/24:00 Beijing, hourly sync check without deploy
- `2e77cd27` ci: tolerate workflow-file push rejections when mirroring upstream master
- `8a99bd98` style: cargo fmt after upstream merge
- `c210187a` ci: add manual verify workflow for remote-only validation
- `f690871e` ci: retry transient SSH preflight failures
- `5f386b59` ci: retry transient SSH preflight failures（重演）
- `1c14be2c/1363912a` i18n: 添加官方升级更新脚本 scripts/update-from-upstream.sh
- `533b92b5/ac1db78f` i18n: 添加全自动更新脚本 auto-update.py
- 核心文件：`.agents/skills/maintain-herdr-fork/*(新增)`、`.github/FORK_AUTOMATION.md(新增)`、`.github/workflows/herdr-cn-sync.yml(新增)`、`herdr-cn-release.yml(新增)`、`herdr-cn-nightly.yml(新增)`、`sync-build-deploy.yml(新增)`、`issue-gate.yml(新增)`、`scripts/sync_upstream.py(新增)`、`scripts/herdr_deploy.py(新增)`、`scripts/auto-update.py(新增)`、`scripts/update-from-upstream.sh(新增)`、`scripts/herdr_automation_issue.py(新增)` 及对应 `test_*.py(新增)`

### G. Nightly 发布修复链（最碎，需合并为一个搬运单元）
功能一句话：nightly 从 Linux 单平台一路修到双平台，修 tag、prerelease、token、release 文件来源。

- `625ab478` fix: drop Windows from nightly build — align with upstream removal (exit code 101)
- `a5eeda6a` fix: restore Windows nightly build with pinned Zig + cache cleanup
- `d4ad9b9f` fix: nightly Linux-only — Windows build fails on windows-2022 runner
- `5f7d4fda` feat: force rebuild on workflow_dispatch by appending GITHUB_RUN_NUMBER to tag
- `6551667c` fix: remove invalid job-level env reference
- `fa3d5fa8` fix: nightly release – 1 preview only, github.token instead of YU_TOKEN
- `dfb1913d` fix: mark nightly release as prerelease so it never steals Latest badge
- `3e48d7e2` fix: re-apply nightly release fixes (make_latest, prerelease, github.token)
- `5cf5225d` fix: herdr-cn-nightly cleanup - add -r to gh api --jq, remove || true
- `bbcc0fe2` fix: nightly release pi-format body + YU_TOKEN cleanup with AND jq filter
- `ca6fca3d` fix: remove .zip from release files (nightly only produces tar.gz)
- `80d1e923` fix: drive release files from resolve output — single source of truth
- `4551af67` feat: add Windows x64 build to nightly (dual-platform release)
- `ac37d8d1` feat: graceful degradation for Windows nightly build
- `e7d8609e/6a7a4e92/53e923c0/dc692d3e/81454146` Merge pull request #1-#5 from yuloop/main-final
- 核心文件：`.github/workflows/herdr-cn-nightly.yml`、`.github/workflows/release.yml`、`justfile`

### H. 配置行为小改
功能一句话：copy-on-select 默认改手动、modal 鼠标穿透保留。

- `49fa2798` feat(config): default copy-on-select to manual
- `4449055b` fix(settings): preserve modal mouse interaction
- 核心文件：`src/selection.rs`、`src/app/input/clipboard.rs`、`src/app/input/modal.rs`、`src/ui/dialogs.rs`、`src/config/model.rs`

### I. 测试可移植性/门禁/文档处理
功能一句话：让维护脚本测试在 Windows 可跑，稳定 sidebar 边界，规范 docs 处理。

- `123f2c08/40af2983` fix(clippy): keep test module last
- `d8a18191/e2d76e97` test(context-menu): select split action by meaning
- `9f335584` fix(ci): enforce Linux Clippy from Windows gate
- `269e5d52` fix(tests): make maintenance checks portable on Windows
- `5403ac6f` fix(tests): stabilize shared sidebar viewport boundary
- `09f7cdbe` fix(tests): align sidebar scroll metrics with separators
- `28f8cff1/8c564a79` fix: normalize cross-platform docs processing
- `686b71a1/43db5ae0` test: run handoff registry regression with runtime
- 核心文件：`scripts/test_*.py`、`scripts/test_docs_translation_parity.py`、`scripts/test_vendor_*.py`、`src/ui/sidebar.rs`、`src/app/input/sidebar.rs`

### J. 文档/website
功能一句话：pane/qwen 部署记录、fork readme 中文化、CN/EN 文档。

- `4396ecef/8686c295` docs: document pane layout and qwen integration
- `0c0b76bb/e364a5be` docs: record pane transfer deployment
- `20040d57/923e42af` docs: localize fork readme
- `218b3b8e` fix: remove README trailing whitespace + graceful 410 handling
- 核心文件：`README.md`、`README.zh-CN.md`、`AGENTS.md`、`docs/next/**`、`website/scripts/*.mjs`

### K. Merge/sync 噪音（29 个，无功能，直接丢弃）
- `ddb22830, bdd1e291, 57f614a4, ccec86c4, 0cbb0915, fe40e65c, d6c18ba0, 4adc69b8, b5b6e0ba, 3659546f, 6baa32a6, 14f12515, ee694457, 9c49dc40, cf548728, 2cd88295, 7aa3e0b0, 82a78943, 19f4b308, 63d2b4ff, 0c30220a, af8f81c0, eec85b70, bf8c222b, cd612c74` 等 `Merge branch ... / merge: ... / chore: merge ...`，rebase 时跳过，以 upstream 为准。

---

## 2) 核心文件 → 上游新位置 + 搬运难度

### 2.1 纯新增、上游未动（易：直接搬运，只需过编译）

| fork 文件 | 上游新位置（HEAD） | 难度 | 说明 |
|---|---|---|---|
| `src/i18n.rs` | 同路径不存在，新建 | 易 | 上游无 i18n，需接 `main.rs`/`cli/spec.rs`/`config.rs` |
| `locales/en.yml, locales/zh.yml` | 同路径不存在，新建 | 易 | 资源文件 |
| `src/app/input/pane_layout.rs` | 同路径不存在，新建；但 `src/app/input/*` 已被上游删除，需改挂到新位置 | 中 | 见 2.3 |
| `src/ui/pane_layout.rs` | 同路径不存在，新建；`src/ui/*` 大重构 | 中 | 需对 `src/ui.rs/sidebar.rs/panes.rs` 新版重接 |
| `src/ipc/windows_listener.rs` | 同路径不存在，新建 | 易 | `src/ipc.rs` 上游未动，接线简单 |
| `src/platform/windows/client_window.rs` | 同路径不存在，新建 | 易 | `windows.rs` 上游小改 60 行 |
| `src/integration/assets/qwen/*` | 同路径不存在，新建 | 易 | 资源 |
| `scripts/auto-update.py, sync_upstream.py, herdr_deploy.py, herdr_automation_issue.py, i18n_key_check.py, update-from-upstream.sh, install_windows_terminal_profile.ps1` | 同路径不存在，新建 | 易 | 独立脚本 |
| `scripts/test_sync_upstream.py, test_herdr_deploy*.py, test_i18n_key_check.py, test_cross_platform_gate.py, test_herdr_automation_issue.py, windows_terminal_profile_test.ps1` | 同路径不存在，新建 | 易 | 独立测试 |
| `.agents/skills/maintain-herdr-fork/*, .github/FORK_AUTOMATION.md, I18N.md` | 同路径不存在，新建 | 易 | 文档/skill |
| `.github/workflows/herdr-cn-*.yml, sync-build-deploy.yml, issue-gate.yml` | 同路径不存在，新建 | 易 | 独立 workflow，注意上游新增 `distribution.yml/website-deploy.yml` 别冲突 |
| `openspec/changes/*（约23个新增）`、`docs/superpowers/reports/*` | 同路径不存在，新建 | 易 | 过程文档，不影响编译 |

### 2.2 改名搬家（中：机械迁移，注意改引用）

| fork 文件 | 上游新位置（HEAD） | 难度 | 说明 |
|---|---|---|---|
| `website/agent-detection/qwen.toml, grok.toml, index.toml` | `distribution/agent-detection/*.toml`（R100） | 中 | 上游 `8a6d6973` 把 website 拆出，内容同名，需搬到 distribution |
| `website/scripts/docs-versions.mjs, prepare-docs.mjs` | `scripts/docs/*.mjs`（R090-R096） | 中 | fork 对旧路径的修改需重放到新路径 |
| `src/agent_resume.rs` | 仍在，但上游新增 `src/app/agent_resume.rs`（`207be3c7`） | 中 | 两个同名文件需合并，fork 改动小（上游未动旧文件） |

### 2.3 被上游删除/大重构（难：必须在新架构上重做，不能直接 cherry-pick）

| fork 文件 | 上游新位置（HEAD） | 难度 | 说明 |
|---|---|---|---|
| `src/app/input/clipboard.rs, mod.rs, modal.rs, mouse.rs, settings.rs, sidebar.rs, terminal.rs` | 已删除（`D src/app/input/*`）。新逻辑在 `src/server/client_shell*.rs`、`src/server/pane_input.rs`、`src/server/client_transport.rs` | 难 | fork 的汉化/点击区域/鼠标/modal 改动全部落在已删文件，`src/app/input/*` 上游 churn 370-4778 行，需按新 client/server 切分重做 |
| `src/app/runtime_mutations.rs` | 已删除（HEAD 无此文件，上游 churn 198 行后删除） | 难 | pane transfer 核心提交 `b6085a34/dc476da9` 挂在这，需并入 `src/app/runtime.rs`（上游改 752 行）或 `src/server/*` |
| `src/ui/dialogs.rs, keybind_help.rs, menus.rs, mobile.rs, navigator.rs, settings.rs, tabs.rs` | 已删除（`D src/ui/*`，`207be3c7 refactor: render the shell in the client`） | 难 | fork 的设置/菜单/移动端改动无载体，需在 client 渲染层重找位置 |
| `src/ui.rs（1513行）、ui/sidebar.rs（3322行）、ui/panes.rs（382行）、ui/widgets.rs（247行）、ui/status.rs（294行）` | 路径不变但上游大改 | 难 | fork 同文件也有改动，cherry-pick 必冲突，需手工三方合并 |
| `src/app/actions.rs（2330行）、api.rs（714行）、api/panes.rs（867行）、mod.rs（4471行）、state.rs（1267行）、runtime.rs（752行）` | 路径不变但上游大改 | 难 | pane transfer/API 相关，需逐 hunk 搬 |
| `src/client/mod.rs（4677行）、client/input/windows_vti.rs（661行）` | 路径不变但上游大改 | 难 | host cursor/Windows VTI 改动需重适配 |
| `src/server/headless.rs（10723行）、server/clients.rs、client_transport.rs` | 路径不变但上游大改/拆分出 `headless/*` | 难 | 与 pane/客户端视图强相关 |
| `src/pty/actor/unix.rs（607行）、pane/terminal.rs（552行）、terminal/runtime.rs（68行）` | 路径不变但上游大改 | 难 | PTY 批量读取需对照新版 actor 重做 |
| `src/api/schema/panes.rs（80行）、response.rs、tests.rs、server.rs` | 路径不变，上游中小改 | 中 | pane API 相关，需小心合并 |
| `src/workspace.rs（170行）、workspace/tab.rs（30行）、persist/snapshot.rs（37行）、selection.rs（64行）、server/autodetect.rs（30行）` | 路径不变，上游小改 | 中 | 冲突面小 |
| `src/config.rs（61行）、config/model.rs（17行）、cli/spec.rs（3行）、main.rs（182行）、update.rs（224行）、build.rs（15行）、Cargo.toml（2行）` | 路径不变，上游小改 | 中 | i18n 接线处，需逐行合 |
| `src/detect/manifests/qwen.toml, detect/manifest.rs, detect/mod.rs, integration/*` | 路径不变，上游未动或小改（`integration/types.rs` 9行） | 易-中 | 最好搬的一批，建议第一棒先搬 |
| `scripts/agent_detection_manifest_check.py（75行）、test_agent_detection_manifest_check.py（66行）` | 路径不变，上游有改 | 中 | 需合并 |
| `docs/next/**（约40个 .mdx/json）、justfile（31行）、AGENTS.md（29行）、.github/workflows/ci.yml（2行）/preview.yml（34行）/release.yml（46行）` | 路径不变，上游小改 | 易-中 | 文档以 fork 为准重放即可，workflow 注意上游新增项别覆盖 |

---

## 3) 给下一棒的搬运顺序建议（仅建议，不执行）

1. 先搬易的：`locales + src/i18n.rs + detect/qwen + integration + scripts新增 + workflows新增 + openspec`，快速恢复编译。
2. 再搬中的：`website→distribution` 改名、`agent_resume` 合并、`config/main/cli` 接线、`schema/workspace` 小改。
3. 最后啃难的：`pane transfer`（`runtime_mutations` + `app/*` + `ui/*`）、`app/input/*` 汉化点击、`ui/*` 大改、`pty` 批量读、`client/server/headless`。每一块单独开分支验证。
4. 直接丢弃：29 个 Merge/sync 噪音提交；nightly 修复链压缩为 1 个提交再搬。

> 本文件为第一棒交付物：只做统计。未改 `src/`，未 merge，未 push。
