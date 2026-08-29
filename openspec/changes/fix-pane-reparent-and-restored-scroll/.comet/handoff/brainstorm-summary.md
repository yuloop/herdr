# Brainstorm Summary

- Change: fix-pane-reparent-and-restored-scroll
- Date: 2026-07-27
- Status: 用户已确认详细技术方案

## 已确认的目标与约束

- 同时提供窗格标题长按拖拽和右键菜单两种入口。
- 已创建并正在运行的持久窗格可以加入已有分屏，也可以脱离为新标签页或新工作区。
- 移动不得重启 PTY、子进程或已知 agent 会话；部署不得丢失远端现场。
- 同标签页现有重新布局和布局模板继续保留。
- 恢复现场后默认位于最新输出；长历史支持普通、按页和端点滚动。
- 不新增外部 API、依赖或持久化格式，不推送官方仓库。

## 确认的技术方案

采用统一 Pane Transfer 状态并复用现有运行时 mutation：

- 扩展当前 `PaneLayoutInteraction`，由一个纯 UI 的 transfer 状态保存稳定来源、候选目标、投放方向和触发来源。
- 同标签页投放调用现有 `layout.rearrange`；跨标签页/工作区和新容器投放调用现有 `pane.move`。
- 标题拖拽、右键菜单和键盘共同使用同一个目标解析器与预览渲染器。
- 提交前不摘除来源窗格；提交时重新解析稳定公共 ID。
- 补强 `pane.move` 的晚期失败恢复，使布局、标签编号、公共窗格身份和焦点可恢复到原位置。

该方案复用现有跨工作区 ID 映射与事件语义，同时满足拖拽、菜单、原子性和现场连续性。直接修改 `Workspace/TileLayout` 会复制运行时语义，只做菜单又不满足长按拖拽需求，均不采用。

### 交互与预览

- 只把实际可见的窗格顶部标题文字区域视为拖拽手柄；边框关闭、标题为空或窗格过窄时不启动拖拽，右键菜单仍可完整操作。
- 左键按下后记录时间和位置；按住约 250ms 且发生至少一格移动后进入 transfer。短按只聚焦窗格，不影响终端内容选择或应用鼠标上报。
- transfer 覆盖层提供：可见窗格四边、所有工作区/标签页列表、目标工作区的新标签页以及新工作区。
- 对同标签页目标渲染现有精确布局预览；对其他标签页使用目标布局的只读几何和目标叶节点生成分屏示意，不切换实际工作区或运行时。
- 拖拽来源在鼠标抬起时提交；菜单来源用 `Enter` 或点击提交；`Esc`、右键和无效区域释放取消。

### 稳定身份与提交

- transfer 状态保存工作区 ID、标签公共编号/ID和窗格公共 ID，不把易变化的列表下标作为提交身份。
- 提交前重新解析来源与目标，过滤同一窗格、已关闭、缩放中或不再属于目标标签页的对象。
- 同标签页目标生成 `LayoutRearrangeParams::Reposition`；跨容器目标生成 `PaneMoveParams`。
- mutation 返回失败或 `changed: false` 时关闭覆盖层并显示本地化 toast；成功后跟随运行时结果聚焦移动后的窗格。

### 事务恢复

- 为 `pane.move` 保存足够的来源快照：工作区身份与位置、标签编号与位置、原布局与焦点、窗格公共编号映射。
- 所有可预期的无效目标在摘除来源前拒绝。
- 若插入阶段仍发生内部失败，将同一个 `MovedPane` 恢复到原标签和原布局，并恢复公共身份与焦点；不关闭 runtime，也不发送退出输入。

### Handoff 重绘

- 每个导入的 `PaneRuntime` 保存一次性的 `handoff_repaint_needed` 标志，而不是服务器级别对所有窗格统一 nudge。
- `initial_history_ansi` 非空并成功回放时清除该标志并将滚动偏移重置为 0；无可用回放画面时保留标志。
- 首个客户端连接仍触发统一检查，但只有标志为真的 runtime 执行缩小/恢复 resize nudge，执行后原子清除。
- 客户端真实最终尺寸变化仍正常 resize 一次，不受上述过滤影响。

### 长历史滚动

- 在 runtime 层增加共享的 host-scrollback 滚轮动作解析，供全应用和 direct attach 调用。
- 无修饰键使用 `ui.mouse_scroll_lines`；Shift 使用 `viewport_rows - 1`；Ctrl、Super/Command 或 Meta 向上设置为最大偏移、向下重置到 0。
- 先判断 `WheelRouting`；`MouseReport` 与 `AlternateScroll` 继续原样转发，绝不应用 host 加速。

## 关键取舍与风险

- 采用延迟提交，牺牲拖动时立即摘除的视觉效果，换取取消与失败安全。
- 长按阈值采用固定 250ms，不新增配置项；过早配置化会扩大文档和兼容范围。
- macOS Command 是否能被识别取决于宿主终端是否报告 Super/Meta 修饰键；Ctrl 始终作为兼容入口。
- `pane.move` 恢复补强触及核心状态，必须先用失败注入测试锁定原行为，再实施。
- 当前分支已有大量未提交布局、复制和 Qwen 修改，所有实现只在这些修改之上增量编辑。

## 测试策略

- 纯函数测试：标题命中、长按阈值、目标解析、边缘方向、滚轮修饰键动作。
- 状态测试：同标签页重排、跨标签/工作区、新标签、新工作区、单窗格来源、空容器清理、取消和目标消失。
- 失败注入测试：插入失败后恢复原标签编号、布局、公共窗格 ID、焦点和同一 terminal/runtime。
- Handoff 测试：有回放只做最终 resize 且偏移为 0；无回放只 nudge 对应窗格；混合窗格不互相影响。
- 输入路由测试：全应用和 direct attach 的普通/Shift/Ctrl/Super 滚轮，以及 mouse-reporting/alternate-scroll 不被劫持。
- 完成目标测试、格式化、Clippy/仓库校验和 Linux release 构建后，备份远端二进制并 live-handoff 部署，比较部署前后 pane ID、根 shell PID、Qwen PID和会话状态。

## Spec Patch

- 在 `interactive-pane-reparenting` 中补充：同标签页标题拖拽使用现有重新布局语义；无可见标题时右键入口仍可用。
- 在 `restored-cli-navigation` 中补充：Command 加速适用于宿主终端能够报告 Super/Meta 修饰键的环境，Ctrl 为通用入口。
