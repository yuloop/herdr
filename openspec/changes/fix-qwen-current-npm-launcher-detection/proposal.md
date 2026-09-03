## Why

Herdr 已具备 Qwen Code 智能体、Hook 和会话恢复支持，但当前官方 npm 包通过 Node 执行 `cli-entry.js` 或 `cli.js`。这些通用文件名无法被现有进程检测映射回 Qwen，导致最新版 Qwen Code 会话不出现在 Herdr 的智能体列表中。

## What Changes

- 根据官方 `node_modules/@qwen-code/qwen-code` 包路径识别 Qwen Code 的当前启动入口。
- 覆盖 Windows、Unix 和 Qwen 自管理更新目录中的启动路径。
- 增加负向回归测试，避免把其他包或 Qwen 包内的非启动脚本误判为智能体。

## Capabilities

### New Capabilities

无。

### Modified Capabilities

无。本次修复让已有 Qwen Code 检测能力覆盖当前官方 npm 启动方式，不改变既有验收要求。

## Impact

- 代码与测试：`src/detect/mod.rs`
- API、配置、依赖和 UI：无变化；检测成功后复用现有智能体聚合与展示链路。
