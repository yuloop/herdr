# herdr

<p align="center">
  <img src="assets/logo.png" alt="herdr" width="100" />
</p>

<p align="center">
  <a href="https://herdr.dev">herdr.dev</a> · <a href="#安装">安装</a> · <a href="https://herdr.dev/zh-cn/docs/quick-start/">快速开始</a> · <a href="https://herdr.dev/zh-cn/docs/">文档</a></p>

<p align="center">
  <a href="https://github.com/herdrdev/herdr/blob/main/README.md">English（上游）</a> · 简体中文
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-666666?labelColor=333333" alt="Apache 2.0 license" /></a>
  <a href="https://yuloop.github.io/herdr"><img src="https://img.shields.io/badge/docs-汉化版-666666?labelColor=333333" alt="汉化版文档" /></a>
</p>

---

> 这是 [herdrdev/herdr](https://github.com/herdrdev/herdr) 的简体中文汉化社区 Fork，非官方项目。
> Copyright © 2025 herdrdev · 基于 [Apache License 2.0](LICENSE) 分发。
> 汉化工作由 yuloop 社区维护，与 herdrdev 官方团队无隶属关系。

https://github.com/user-attachments/assets/043ec09f-4bdd-41d5-aee0-8fda6b83e267

**智能体复用器，住在你的终端里。）

- 每个智能体一目了然——blocked、working、done。真实的终端视图，而不是包装过的转述。
- 分离后智能体继续运行——从任意终端重新连接，或通过 ssh。会话在重启后依然保留。
- 智能体也能使用 herdr——纯 socket api：智能体可以创建窗格、读取输出、互相等待。[智能体技能 →](https://herdr.dev/zh-cn/docs/agent-skill/)
- 键盘和鼠标都是一等公民——tmux 风格的前缀键，以及点击、拖动、分割。按当下的场景选择，而不是被工具锁死。
- 插件——扩展窗格和工作流。[浏览插件市场 →](https://herdr.dev/plugins/)
- 单个 rust 二进制，没有 electron——运行在你已经在用的任何终端里。

---

## 汉化版与官方版差异

| 维度 | 官方版（herdrdev/herdr） | 本仓库（汉化版） |
|---|---|---|
| 界面语言 | 纯英文 | 简体中文（458 行翻译词表） |
| 国际化 | 无 | rust-i18n 编译时嵌入 |
| 构建自动化 | 无 | 4次/天自动构建（Nightly）+ 每日稳定版 |
| 一键安装 | install.sh | `herdr cn-install` |
| 预览版 | 无 | Nightly 预览版（跟随上游 dev 分支） |
| 中文文档 | 无 | 全量文档中文化（6 版本×18 篇） |
| 汉化文件 | 无 | locales/zh.yml + zh-CN.md + docs/zh-cn/* |

> herdr 汉化版基于上游 clean master rebuild，仅 8 个源文件 + 5 个文档/工作流与上游不同。
> 具体本地修改文件：
> - `locales/zh.yml`（458 行中文翻译）
> - `src/i18n.rs` + `build.rs`（国际化集成）
> - `src/integration/{mod,registry,targets}.rs`（Qwen + TUI plugin）
> - `README.zh-CN.md`、`README.md`（汉化首页）
> - `.github/workflows/herdr-cn-{sync,release,nightly}.yml`

## 安装

```bash
curl -fsSL https://herdr.dev/install.sh | sh
```

汉化版一键安装：

```bash
herdr cn-install
```

或手动下载发布二进制解压到 PATH。

运行智能体，分割窗格，走开。ctrl+b q 分离会话，herdr 重新连接。[快速开始 →](https://herdr.dev/zh-cn/docs/quick-start/)

## 文档

全部文档在 [herdr.dev/docs](https://herdr.dev/docs/)：[快速开始](https://herdr.dev/zh-cn/docs/quick-start/) · [概念](https://herdr.dev/zh-cn/docs/concepts/) · [支持的智能体](https://herdr.dev/zh-cn/docs/agents/) · [键盘快捷键](https://herdr.dev/zh-cn/docs/keyboard/) · [配置](https://herdr.dev/zh-cn/docs/configuration/) · [会话状态](https://herdr.dev/zh-cn/docs/session-state/) · [远程](https://herdr.dev/zh-cn/docs/persistence-remote/) · [集成](https://herdr.dev/zh-cn/docs/integrations/) · [插件](https://herdr.dev/zh-cn/docs/plugins/) · [Socket API](https://herdr.dev/zh-cn/docs/socket-api/)

## 致谢

所有赞助者和支持者见 [SPONSORS.md](./SPONSORS.md) — 谢谢 🐑

企业合作：hey@herdr.dev

## 智能体指引

如果你是 AI 智能体协助此仓库，请在修改前阅读 [AGENTS.md](./AGENTS.md)，在提交 issue 或 PR 前阅读 [CONTRIBUTING.md](./CONTRIBUTING.md)。

## 开发

```bash
git clone https://github.com/yuloop/herdr
cd herdr
cargo build --release

just test        # 单元测试
just check       # 格式化、测试和维护检查
```

## 许可证

Herdr 源代码基于 [Apache License 2.0](LICENSE) 分发。
汉化版本继承相同许可证，Copyright © 2025 herdrdev，汉化维护 © 2025 yuloop。
