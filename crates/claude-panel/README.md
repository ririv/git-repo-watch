# Claude Code 里的 gw 面板

在 Claude Code 终端会话输入 `/gw`，可以查看会话当前目录所在的 Git 仓库。面板复用 gw 的仓库概览、工作区差异和提交列表。面板内的操作只读。

## 启动

在 git-repo-watch 的源码目录构建一次。

```sh
cargo build --release --locked -p gw-claude-panel
export GW_CLAUDE_PANEL="$PWD/target/release/gw-claude-panel"
export GW_CLAUDE_MOD="$PWD/crates/claude-panel/mod"
```

切换到要查看的 Git 仓库，再启动 Claude Code 终端版。原型验证使用 Claude Code 2.1.330。

```sh
claude --plugin-dir "$GW_CLAUDE_MOD"
```

输入 `/gw`。按 `1`、`2`、`3` 切换概览、改动和历史，按 `j`、`k` 切换文件或提交，按 `r` 重新读取，按 `Esc` 关闭面板。也可以点击相应的面板控件。

面板读取 gw 的默认配置。如平时使用 `gw --home`，可以在启动 Claude Code 前把 `GW_CLAUDE_HOME` 设为同一个已初始化配置目录的绝对路径。

## 实现

Rust 进程用已有的 Ratatui 视图生成一帧。Mod 在需要新画面时运行该进程，用 Claude Code 的 `Raster` 绘制单格字符，用 `Text` 绘制中文等双宽字符和带字形样式的行。原型中的历史视图显示提交列表，像素提交图及完整的 gw 键鼠交互仍由独立终端界面提供。

## 验证

```sh
claude plugin validate crates/claude-panel/mod
claude plugin test crates/claude-panel/mod
```
