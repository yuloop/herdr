# Comet Design Handoff

- Change: fix-pane-reparent-and-restored-scroll
- Phase: design
- Mode: compact
- Context hash: 119aa838f272c14549940b44ebe078f98260ed8a5bbbd171a4032c68c3dcfb7d

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## openspec/changes/fix-pane-reparent-and-restored-scroll/proposal.md

- Source: openspec/changes/fix-pane-reparent-and-restored-scroll/proposal.md
- Lines: 1-30
- SHA256: 189c35e821e07e6c563e58ab1ab77bd426067ebbf19c2b88668f97de84ad3029

```md
## Why

当前分屏界面只能重排同一标签页内已经存在的窗格，无法把已创建的窗格加入其他分屏，也没有从分屏中脱离为标签页或工作区的交互入口。与此同时，现场 CLI 在热交接和长历史场景下会产生额外重绘，默认滚动步长又过小，恢复到最新内容或浏览长历史都很费力。

## What Changes

- 为现有窗格增加完整的交互式移动能力：标题拖拽、目标边缘分屏投放、脱离为新标签页或新工作区。
- 在窗格右键菜单中提供同等能力的备用入口，保证不依赖拖拽也能完成移动。
- 跨标签页、跨工作区移动时复用现有 `pane.move` 运行时语义，保持 PTY、进程和会话连续。
- 优化现场 CLI 热交接的首次重绘，避免对主屏 CLI 进行不必要的双重尺寸抖动。
- 为长历史提供快速翻页和快速回到底部的交互，同时保留可配置的普通滚轮步长。
- 增加窗格移动、取消/失败回滚、现场恢复及长历史滚动的回归测试。

## Capabilities

### New Capabilities

- `interactive-pane-reparenting`: 现有窗格通过拖拽或右键菜单加入目标分屏、脱离为标签页/工作区，并保持运行中会话连续。
- `restored-cli-navigation`: 现场 CLI 恢复时减少破坏性重绘、默认呈现最新内容，并支持高效导航长历史。

### Modified Capabilities

<!-- 当前仓库没有对应的主规格；本次不修改已有 capability。 -->

## Impact

- 影响 TUI 状态、鼠标输入、右键菜单、窗格布局覆盖层及本地化文案。
- 复用现有 pane move API 和 workspace 移动逻辑，不新增对外协议方法。
- 影响 Unix live handoff 的首次重绘策略，以及全应用和 direct-attach 的滚动输入。
- 不引入新依赖，不改变持久化数据格式，不终止或重启被移动窗格中的子进程。
```
## openspec/changes/fix-pane-reparent-and-restored-scroll/design.md

- Source: openspec/changes/fix-pane-reparent-and-restored-scroll/design.md
- Lines: 1-100
- SHA256: 8a1d8255a103cb4e67375e8223efcbce2c754e642f3cbad8a0d34e99da09584b

[TRUNCATED]

```md
## Context

Herdr 已有两套相邻但未连通的能力：

1. `layout.rearrange` 和当前 `PaneLayoutInteraction` 只能修改同一标签页内的布局。
2. `pane.move` 已能把运行中的窗格移动到已有标签页、新标签页或新工作区，并处理跨工作区 ID 映射、空来源标签页/工作区清理和失败恢复。

因此，窗格无法“脱离”或加入其他分屏的根因在 TUI 交互层，而不是运行时缺少移动语义。现场 CLI 的问题则集中在 Unix live handoff：导入后的每个 PTY 会在首个客户端连接时做一次缩小再恢复的尺寸抖动。对使用主屏绘制完整对话的 CLI，这会触发额外全量重绘；同时普通滚轮默认每格仅滚动三行，长历史导航成本很高。

当前工作区还包含本分支既有的布局、侧栏、复制和 Qwen 集成修改。本变更必须以这些修改为基线，不覆盖或回退它们。远端部署必须保留现有 PTY 和子进程。

## Goals / Non-Goals

**Goals:**

- 让已创建且正在运行的窗格可以通过标题拖拽或右键菜单加入任意已有分屏。
- 让分屏窗格可以脱离为新标签页或新工作区，同时保持 PTY、进程、cwd、agent 会话和输出连续。
- 所有移动在释放/确认前只显示预览；取消或失败不得改变来源布局。
- 减少 live handoff 对已有主屏 CLI 的重复重绘，并保证恢复后默认显示最新内容。
- 为全应用和 direct-attach 模式提供一致的长历史快速导航。
- 用现有 `pane.move` 作为唯一运行时移动实现，不新增外部协议方法。

**Non-Goals:**

- 不实现跨 Herdr server、跨操作系统进程或跨机器拖拽。
- 不支持把弹出式 popup pane 直接转成持久窗格。
- 不持久化一次尚未完成的拖拽手势。
- 不改变普通滚轮步长配置的含义，也不截断用户已有滚动历史。
- 不重写布局树或 pane move 的底层数据模型。

## Decisions

### 1. 以统一的 Pane Transfer 交互连接 UI 与现有 `pane.move`

扩展当前窗格布局交互状态，增加只描述用户意图的 transfer 状态：来源窗格、当前目标、投放边和交互来源（标题拖拽或右键菜单）。渲染层只读取该状态画预览；输入层只更新目标；最终提交由 `App` 的运行时 mutation 包装调用现有 `handle_pane_move` 语义。

这样可以避免在鼠标代码中复制 workspace/tab/pane 移动逻辑，也能继续使用现有失败恢复和事件发布。备选方案是在鼠标释放时直接修改 `Workspace`，但它会绕过 API 已覆盖的跨工作区 ID 映射与回滚，因此不采用。

### 2. 标题拖拽只在明确的窗格标题区域激活

左键按下窗格上边框标题区域后记录 press 状态；达到短暂按住阈值并开始拖动时进入 transfer 模式。终端内容区中的选择、鼠标上报和滚动不受影响。没有可见标题边框时，右键菜单仍是完整备用入口。

拖动期间提供四类目标：

- 可见窗格的上/下/左/右边缘：移动到该窗格所在标签页，并按对应方向组成分屏。
- 标签项：移动到该标签页，以其当前聚焦窗格为锚点并默认向右分屏。
- “新标签页”投放区：在目标工作区创建新标签页。
- “新工作区”投放区：创建新工作区并把来源窗格作为首个窗格。

悬停其他工作区或标签项时允许切换预览上下文，来源窗格在提交前仍留在原位。`Esc`、鼠标在无效区域释放或来源/目标在交互期间消失都只取消操作。

备选方案一是只做右键菜单，改动较小但不满足标题拖拽。备选方案二是拖动中即时从来源布局摘除窗格，视觉更直接但取消和异常恢复风险更高。采用“延迟提交 + 纯预览”以保证运行中会话安全。

### 3. 右键菜单复用同一目标模型

窗格右键菜单增加“移动或脱离”入口，打开与拖拽相同的 transfer 覆盖层。鼠标可点击目标；键盘可循环目标和投放边，`Enter` 确认、`Esc` 取消。现有“同标签页内重新布局”和布局模板继续保留，二者职责分别是重排布局树和跨容器移动窗格。

### 4. 首次 handoff 重绘按窗格是否已有可用画面选择

导入 runtime 时记录该窗格是否通过 `initial_history_ansi` 获得了可显示的主屏快照：

- 有快照：显式回到底部；首个客户端按真实最终尺寸正常 resize，但不再执行缩小再恢复的 repaint nudge。
- 无快照（例如恢复 alternate-screen 应用或需要 agent 自己重绘）：保留现有一次 repaint nudge。

这能让 Qwen 等主屏 CLI 避免额外全量转录重绘，同时不让无快照应用恢复成空白。备选方案是完全删除 nudge，但会使部分 alternate-screen 或无历史窗格首帧为空；另一个备选是统一保留 nudge 并只增大滚动速度，它不能解决重复历史，因此均不采用。

### 5. 长历史导航使用修饰键加速，不改变普通滚轮

仅在 Herdr 接管的 host scrollback 路径中应用：

- 普通滚轮：继续使用 `ui.mouse_scroll_lines`。
- `Shift + 滚轮`：按一个可见页面滚动。
- `Ctrl + 滚轮` 或 macOS `Cmd + 滚轮`：向上跳到最旧位置，向下直接回到最新位置。

全应用和 direct-attach 共享相同语义。若窗格应用请求 mouse reporting 或 alternate scroll，滚轮及修饰键仍转发给应用，不劫持其交互。

### 6. 失败保持原子性并给出可见反馈

提交前重新验证来源和目标。运行时移动成功后才清理交互状态并切换焦点；失败时依赖现有 `pane.move` 恢复来源窗格，关闭覆盖层并显示错误 toast。任何路径都不得关闭 PTY 或发送退出输入。

```

Full source: openspec/changes/fix-pane-reparent-and-restored-scroll/design.md

## openspec/changes/fix-pane-reparent-and-restored-scroll/tasks.md

- Source: openspec/changes/fix-pane-reparent-and-restored-scroll/tasks.md
- Lines: 1-19
- SHA256: 092632c754fb667fbd33cc5b17c2c5a566672d9e20f87a867dbb3fedd16dd5a0

```md
## 1. Characterization and transfer core

- [ ] 1.1 Add failing characterization tests for title-only drag activation, transfer cancellation, target resolution, and reuse of existing pane move semantics
- [ ] 1.2 Add the stable pane-transfer interaction state and an App runtime mutation wrapper that commits through existing `pane.move` behavior

## 2. Pane transfer interaction

- [ ] 2.1 Implement long-press pane-title dragging, pane-edge/tab/new-tab/new-workspace drop targets, pure preview rendering, and invalid-drop cancellation
- [ ] 2.2 Add the right-click move-or-detach fallback, keyboard navigation, localized labels, error toast handling, and cross-workspace regression coverage

## 3. Restored CLI navigation

- [ ] 3.1 Add per-pane handoff repaint eligibility, reset replayed panes to live output, and skip redundant resize nudges when a usable screen was restored
- [ ] 3.2 Implement page-sized and endpoint modifier scrolling for full-app and direct-attach host scrollback while preserving application-owned wheel routing

## 4. Validation and deployment

- [ ] 4.1 Run targeted tests, formatting/lint checks, the repository validation recipe, and a Linux release build; record any scoped exclusions
- [ ] 4.2 Back up and live-handoff deploy the validated binary to `192.168.31.4`, then verify pane IDs, child PIDs, agent sessions, layout moves, and scroll continuity without submitting upstream
```

## openspec/changes/fix-pane-reparent-and-restored-scroll/specs/interactive-pane-reparenting/spec.md

- Source: openspec/changes/fix-pane-reparent-and-restored-scroll/specs/interactive-pane-reparenting/spec.md
- Lines: 1-76
- SHA256: 46716d6edc5672eb0d9825c8d1a3f6a0aa430b3ac6246ded8ca301ae3e21198f

```md
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
```

## openspec/changes/fix-pane-reparent-and-restored-scroll/specs/restored-cli-navigation/spec.md

- Source: openspec/changes/fix-pane-reparent-and-restored-scroll/specs/restored-cli-navigation/spec.md
- Lines: 1-63
- SHA256: 38468f9f2b4d3a9fb896b36b0c41a76b903bf43323c26b3f67567aef99ec930d

```md
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
```
