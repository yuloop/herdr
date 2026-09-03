# Qwen Code 当前 npm 启动器兼容验证

- Change：`fix-qwen-current-npm-launcher-detection`
- 基线：`074fb8ace1ef8558f5656070505a70f53f79b2a0`
- 验证模式：light
- 结论：PASS

| 检查项 | 结果 | 证据 |
| --- | --- | --- |
| 任务完成 | OK | `tasks.md` 3/3 已勾选 |
| 改动范围 | OK | 实现及测试仅修改 `src/detect/mod.rs`；其余为本次流程产物 |
| 编译 | OK | Zig 0.15.2 环境下 `cargo build`、`cargo check --bin herdr --locked` 通过 |
| 相关测试 | OK | `cargo test --bin herdr qwen -- --nocapture`：13 passed，0 failed |
| 安全检查 | OK | 无新增 `unsafe`、依赖或凭据；仅对白名单官方包路径和入口进行匹配 |
| 代码审查策略 | OK | `.comet.yaml` 为 `review_mode: off`，按 hotfix 默认策略跳过自动审查 |

补充检查：

- `cargo clippy --bin herdr --locked -- -D warnings` 通过。
- `cargo fmt --all -- --check` 通过。
- `git diff --check` 通过。
- 正向用例覆盖 Windows npm shim、Qwen 自管理 0.21 更新目录、Unix 源码入口和 `--expose-gc` 子进程参数。
- 负向用例覆盖其他 npm 包、Qwen 包内构建脚本、嵌套依赖和相似包名。
