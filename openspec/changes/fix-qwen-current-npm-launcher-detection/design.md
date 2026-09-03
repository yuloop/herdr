## Context

Qwen Code 的 npm 启动器进程名是 `node`，Herdr 因此需要从 argv 中的脚本路径恢复智能体身份。现有通用 basename 检测能识别旧路径中的 `qwen.js`，却无法识别当前 scoped package 使用的 `cli-entry.js` 和 `cli.js`。

## Goals / Non-Goals

**Goals:**

- 识别官方 Qwen Code npm 包的当前运行入口。
- 同时支持 Windows、Unix 和版本化自管理更新路径。
- 将匹配限制在官方包根目录及已知入口，控制误判。

**Non-Goals:**

- 改变 Qwen Hook、会话恢复、屏幕清单或 UI。
- 识别任意名为 `cli.js` 的 Node 程序。

## Decisions

1. 继续在 `agent_name_from_known_package_path` 中处理 Node 包路径，与现有 Pi 包装器识别保持同一职责边界。
2. 先定位连续的 `node_modules/@qwen-code/qwen-code` 包根，再仅接受根目录 `cli-entry.js`、`cli.js`，以及 `scripts/cli-entry.js`、`dist/cli.js`、`dist/index.js`。
3. 路径沿用已有的大小写归一化及 Windows/Unix 分隔符处理。
4. 使用正向表驱动测试覆盖当前入口，并用其他包和非入口脚本验证不会泛化匹配。

## Risks / Trade-offs

- [Qwen 后续更换入口] → 继续按官方包内的明确入口增量维护，不放宽为通用文件名匹配。
- [开发构建入口与发布入口不同] → 同时覆盖当前发布启动器和仓库构建产物入口。
