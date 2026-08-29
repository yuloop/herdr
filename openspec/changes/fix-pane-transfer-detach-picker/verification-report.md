# 窗格移动/脱离目标选择器验证报告

- 日期：2026-07-27
- 变更：`fix-pane-transfer-detach-picker`
- 验证模式：light
- 实现范围：`src/app/state.rs`、`src/ui/pane_layout.rs`

## 六项检查

| 检查项 | 结果 | 证据 |
| --- | --- | --- |
| tasks 全部完成 | PASS | `tasks.md` 3/3 已勾选 |
| 改动范围与任务一致 | PASS | 两个实现文件分别负责候选排序和目标标签/命中测试 |
| 编译通过 | PASS | `cargo check --locked --bin herdr`、`cargo build --locked --bin herdr` |
| 相关测试通过 | PASS | 隔离串行窗格布局测试 16/16；多分屏候选顺序 1/1；右键入口与标题拖拽保护等相关测试通过，共 19 个不重复用例 |
| 无明显安全问题 | PASS | scoped diff 未新增 `unsafe`、凭据字面量、依赖或外部接口 |
| 代码审查策略 | PASS（按配置跳过） | `.comet.yaml` 为 `review_mode: off`，未自动派发代码审查 |

## 回归证据

- 修复前，`pane_transfer_detach_targets_stay_ahead_of_many_split_moves` 失败：第一个候选是窗格边缘移动目标。
- 修复前，`transfer_labels_include_user_context_and_stable_pane_identity` 失败：标签仅为 `w1:p1 · ←`。
- 修复后，14 个候选的溢出场景中，首行可点击“脱离到新标签页”，次行可点击“脱离到新工作区”。
- 两个同名窗格的目标行同时包含工作区、标签页、窗格标题和各自稳定 pane ID。
- 现有同标签重排、跨工作区移动、脱离为新标签页/新工作区及失败回滚测试均通过。
- `cargo fmt`、`git diff --check` 和 `openspec validate fix-pane-transfer-detach-picker --strict` 均通过。

## 全量测试诊断说明

曾尝试在 Windows 单进程内并行执行全部 2696 个二进制测试。该运行读取了真实用户目录中的旧 Codex manifest 缓存，并在共享 cwd/环境的进程型测试上停止推进，因此不作为本变更验收结论。代表性 Codex manifest 失败项在空的 `XDG_CONFIG_HOME/XDG_STATE_HOME` 下串行复跑通过；一个停止推进的导航测试也在相同隔离环境下立即通过。当前变更未修改检测、配置或工作区创建模块。

## 结论

实现与本次热修复规格一致，构建和相关回归测试通过。技术验证结论为 **PASS**；分支处理仍待 `finishing-a-development-branch` 能力可用后由用户选择，尚未写入 `branch_status: handled`。
