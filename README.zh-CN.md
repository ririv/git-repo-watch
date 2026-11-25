# Rivet Git

[English](README.md) · 简体中文

在一个终端里管理多个 Git 仓库。按 `/` 就能找到需要的命令。

Rivet Git（`rvgit`）支持 macOS、Linux 和 Windows。

## 一眼看出哪些仓库需要处理

可以添加单个仓库，也可以扫描一个目录，勾选要跟踪的仓库。首页集中显示哪些工作区有改动、当前分支比上游领先或落后多少，以及哪些仓库读取或获取远程更新时失败了。

用 `/repos fetch` 批量获取已登记仓库的远程更新，运行期间可以继续浏览，结果会保留到下次启动。获取更新时，本地分支和工作区文件保持原样。领先和落后数量基于本地的远程跟踪引用，获取更新后会按新引用计算。

进入仓库后，可以查看差异和提交历史，按文件、变更块或行暂存，确认后提交或推送。输入 `/repos` 就能回到总览。

## 随时找得到命令

按 `/` 查看当前界面的可用命令，可以搜索命令名或中英文说明。例如，在差异预览里按 `/` 再输入「布局」，就能找到显示选项。切换分支时，也可以从候选列表里选择本地分支。

用 `↑` 和 `↓` 浏览候选，`Tab` 补全，`Enter` 打开命令或应用选项。要跨界面搜索所有命令，输入 `//`。

## 用 npm 安装

安装 Git 和 Node.js 20 或更新版本后，运行：

```sh
npm install -g @efct/rvgit
rvgit repos
```

npm 包提供 macOS（Apple 芯片和 Intel）、Linux（x64）、Windows（x64）的预编译程序。

也可以从 [GitHub Releases](https://github.com/ririv/rvgit/releases/latest) 下载预编译程序。

## 从源码安装

先安装 Git，并通过 Rustup 安装 Rust 工具链。在终端中克隆本项目。

```sh
git clone https://github.com/ririv/rvgit.git
cd rvgit
```

### macOS 和 Linux

macOS 需要安装 Xcode Command Line Tools。Linux 需要安装 C 编译器、`pkg-config` 和 OpenSSH 客户端。然后运行：

```sh
./install.sh
```

如果 `~/.local/bin` 还未加入 `PATH`，请先加入，再运行 `rvgit repos`。

### Windows

安装 MSVC C++ Build Tools，并确认 Git for Windows 已加入 `PATH`。在项目目录中打开 PowerShell，运行：

```powershell
cargo build --locked --release --package app --no-default-features
.\target\release\rvgit.exe repos
```

要将命令安装到本机，运行 `.\target\release\rvgit.exe install`，再把 `%LOCALAPPDATA%\gw\bin` 加入用户的 `PATH`。

## 快速上手

运行 `rvgit repos` 并按界面提示完成初始化。然后在首页输入栏使用这些命令：

| 命令 | 用途 |
| --- | --- |
| `/repos add <path>` | 添加一个仓库。 |
| `/repos scan <path>` | 扫描目录并选择要添加的仓库。 |
| `/repos filter problems` | 只显示需关注或失败的仓库。 |
| `/repos fetch` | 获取已登记仓库的远程更新。 |

选中仓库后按 `Enter` 打开。在 Git 仓库目录里运行 `rvgit` 也能直接进入该仓库。按 `F1` 打开帮助。

界面语言跟随系统。需要手动选择时，在启动前将 `APP_LANG` 设为 `en`（英文）或 `zh`（简体中文）。

## 许可证

[MIT](LICENSE) · [第三方许可](THIRD_PARTY_NOTICES.md)
