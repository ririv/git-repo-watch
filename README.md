# Rivet Git

English · [简体中文](README.zh-CN.md)

Manage multiple Git repositories from one terminal. Press `/` to find the command you need.

Rivet Git (`rvgit`) runs on macOS, Linux, and Windows.

## See which repositories need attention

Add a repository or scan a folder and choose which repositories to track. The dashboard shows which working trees have changes, which checked-out branches are ahead or behind their upstreams, and which repositories failed to load or fetch.

Fetch updates for all tracked repositories in one batch with `/repos fetch`. You can keep browsing while it runs. Results are saved between sessions, and fetching leaves your local branches and files as they are. Ahead and behind counts use Git's local remote-tracking references, which fetch updates.

Open a repository to review its diffs and history, stage changes by file, hunk, or line, and commit or push when ready. The dashboard is always a `/repos` command away.

## Find commands as you work

Press `/` to see commands for the view you're in. Search by command name or by its English or Chinese description. In a diff, for example, press `/` and type `layout` to find the display options. When switching branches, you can choose from the available local branches.

Use `↑` and `↓` to browse, `Tab` to complete, and `Enter` to open a command or apply a choice. Type `//` to search across all command groups.

## Install with npm

With Git and Node.js 20 or newer installed, run:

```sh
npm install -g @efct/rvgit
rvgit repos
```

The npm package includes prebuilt executables for macOS (Apple silicon and Intel), Linux (x64), and Windows (x64).

Prebuilt binaries are also available from [GitHub Releases](https://github.com/ririv/rvgit/releases/latest).

## Install from source

Install Git and the Rust toolchain with Rustup. Then open a terminal and clone this repository.

```sh
git clone https://github.com/ririv/rvgit.git
cd rvgit
```

### macOS and Linux

On macOS, install the Xcode Command Line Tools. On Linux, install a C compiler, `pkg-config`, and the OpenSSH client. Then run:

```sh
./install.sh
```

Add `~/.local/bin` to your `PATH` if needed, then run `rvgit repos`.

### Windows

Install the MSVC C++ Build Tools and make sure Git for Windows is on your `PATH`. From the checkout, run in PowerShell:

```powershell
cargo build --locked --release --package app --no-default-features
.\target\release\rvgit.exe repos
```

To install the command, run `.\target\release\rvgit.exe install` and add `%LOCALAPPDATA%\gw\bin` to your user `PATH`.

## Get started

Run `rvgit repos` and follow the setup prompts. In the dashboard's input bar, use these commands:

| Command | What it does |
| --- | --- |
| `/repos add <path>` | Track one repository. |
| `/repos scan <path>` | Find repositories in a folder and choose which ones to track. |
| `/repos filter problems` | Show repositories that need attention and repositories with errors. |
| `/repos fetch` | Fetch updates for tracked repositories. |

Select a repository and press `Enter` to open it. Running `rvgit` inside a Git repository also opens it directly. Press `F1` for help.

The interface follows your system language. Set `APP_LANG=en` or `APP_LANG=zh` before launch to choose English or Simplified Chinese.

## License

[MIT](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)
