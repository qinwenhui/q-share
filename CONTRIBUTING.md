# 贡献指南

感谢你愿意帮助 q-share 变得更好！

## 环境

- Rust 1.88+
- Node 20+

## 构建与测试

```bash
just web-install   # 首次：安装前端依赖（或 cd web && npm install）
just build         # release 构建（内嵌前端产物）
just test          # cargo test --workspace
just check         # cargo fmt + cargo clippy（-D warnings）
```

构建产物：`cargo build -p qshare-gui` → `target/release/qshare`（GUI），
`cargo build -p qshare-cli` → `target/release/qshare-cli`，TUI 对应 `qshare-tui`。

## 仓库布局

```
crates/
├── qshare-core     # axum 服务端核心：sandbox、目录缓存、watcher、缩略图、mDNS、统计
├── qshare-cli      # 无头 CLI（clap）
├── qshare-gui      # iced 原生 GUI（macOS / Windows）
├── qshare-tui      # ratatui 终端 UI
└── qshare-assets   # rust-embed 打包 web/dist
web/                # SolidJS + Vite + TypeScript 前端
```

## 提 PR 之前

1. `cargo fmt --all -- --check` 通过
2. `cargo clippy --workspace --all-targets -- -D warnings` 通过
3. `cargo test --workspace` 通过
4. 改到前端的话跑 `cd web && npm run build`
5. 在 PR 描述里说清楚改动**为什么**值得做——这个仓库的注释很看重"为什么"而不是复述代码

## 其他约定

- 一个 PR 只做一件事，保持改动聚焦。
- 无强制提交信息规范，但请让提交信息能说清目的。
- 注释风格：跟仓库已有代码保持一致，优先解释"为什么"，不解释显而易见的事。
