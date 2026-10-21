# gw

面向 macOS 和 Linux 的多仓库 Git 状态检查、受控推送和终端管理工具。直接运行 `gw` 进入 TUI；`gw status` 查询已有结果。系统任务每六小时执行一次，完成后退出，期间没有常驻服务，也不依赖 Agent。

项目使用 Rust。本地仓库能力通过 libgit2 实现；fetch、commit、push 和服务器引用查询固定使用系统 Git，保留网络认证、凭据助手、钩子与提交签名行为。两类后端按职责组合，不在失败时自动换用另一种实现。

## 安装

需要 Rust 和 C 编译工具链：macOS 使用 Xcode Command Line Tools；Linux 使用 GCC 或 Clang，并要求 systemd 用户服务管理器。项目工具链固定在 `rust-toolchain.toml`，依赖固定在 `Cargo.lock`。构建时会编译内置 libgit2，不要求用户安装 Go、Python 或独立 libgit2 运行库。系统 Git 和 SSH 仍是网络及提交操作的必要依赖。

```sh
cp config.example.json config.json
# 按下面的配置说明编辑 config.json。
./install.sh
```

macOS 安装位置：

- `~/.local/bin/gw`：原生可执行文件。
- `~/Library/Application Support/gw/location.json`：安装目录和二进制 SHA-256。
- `~/Library/LaunchAgents/local.git-repo-watch.plist`：用户级 launchd 系统任务。

将 `~/.local/bin` 加入 PATH 后运行 `gw`。项目内也可使用 `./gw`。安装核验现有命令的归属，原子替换可执行文件，并应用配置中的检查间隔。`RunAtLoad=false`，安装和重新加载任务都不会立即触发检查。访问可移动卷需要 macOS 授权；勿扰模式会影响通知显示。

```sh
./build.sh          # 构建到 dist/gw，不安装或检查
./gw status        # 项目内查询已有结果
./uninstall.sh     # 卸载系统任务，保留数据和管理命令
```

Linux 安装使用同一个 `./install.sh`。Debian/Ubuntu 构建依赖可执行：

```sh
sudo apt-get install build-essential pkg-config git openssh-client libnotify-bin
```

Linux 安装位置：

- `~/.local/bin/gw`：原生可执行文件。
- `$XDG_DATA_HOME/gw/location.json`：安装定位，未设置时使用 `~/.local/share/gw/`。
- `$XDG_CONFIG_HOME/systemd/user/local.git-repo-watch.service`、`.timer`：用户任务，未设置时使用 `~/.config/systemd/user/`。

XDG 路径必须为绝对路径。安装需能连接 `systemctl --user`，不自动改用 cron。只启用 timer，首轮在配置间隔后运行；每轮是一次性进程，结束后退出。默认随用户会话管理；若需要退出登录后仍运行，可由用户显式执行 `loginctl enable-linger`。日志使用 `journalctl --user -u local.git-repo-watch.service` 查看。自动推送失败通知通过 `notify-send` 发送，需要桌面通知服务和用户会话 D-Bus；无桌面的服务器不能显示通知，失败会写入报告。

Windows 后端尚未实现，当前明确拒绝编译。调度、通知与终端启动分别按平台隔离；后续 Windows 还需实现任务计划、进程树管理、文件锁和权限语义。

构建脚本对 Rust 和 C 依赖执行源码路径裁剪。`config.json`、`var/`、`target/`、`dist/`、日志和定位信息不进入版本控制。

## 界面语言

支持中文和英文。首次显示前读取语言偏好，同一次运行期间保持一致；macOS 首选语言为中文（含简繁体区域标识）时使用中文，其余语言默认英文。当前中文文案为简体中文。后台任务使用同一套系统语言检测，不依赖终端常驻。

优先级为 `APP_LANG` → 非空 `LC_ALL` → 非空 `LC_MESSAGES` → macOS 首选语言 → `LANG` → 英文。终端的 `LANG` 不会覆盖 macOS 明确的系统首选语言；`LC_ALL` 和 `LC_MESSAGES` 视为用户显式指定的消息语言。

```sh
APP_LANG=en gw          # 本次以英文打开 TUI
APP_LANG=zh gw          # 本次以中文打开 TUI
APP_LANG=en gw status   # 本次以英文查询
```

`APP_LANG` 只接受 `en` 或 `zh`，其他值明确报错。未设置时自动检测；切换系统语言后重新运行 `gw` 生效。语言选择不修改现有配置格式、JSON 字段名、枚举值或报告数据。路径、分支名、用户提交说明、Git 原始错误与历史报告中保存的原始说明保留原文。

文案通过 `i18n::Message` 标识集中维护，两种语言都必须显式提供；格式化消息在编译期检查参数。异步任务显式继承当前语言，界面不会因线程切换而混用语言。

## 配置

配置保存在安装目录中的 `config.json`：

```json
{
  "interval_seconds": 21600,
  "repositories": [],
  "excluded_roots": [],
  "push_policy": "disabled",
  "max_concurrency": 4
}
```

仓库与排除目录填写规范绝对路径。仓库列表显式维护，检查不会重新扫描发现目录。已排除目录下的仓库不能注册。

- `interval_seconds`：检查间隔，至少 60 秒。修改后运行 `gw install` 应用系统调度。
- `repositories`：仓库根目录列表，包括通过 `.git` 文件关联的工作区与子模块。
- `excluded_roots`：排除目录；其下的注册项会在排除操作时移除。
- `push_policy`：`disabled` 只检查；`fast_forward` 允许受控自动推送。
- `max_concurrency`：并发检查上限，1 至 32。旧配置缺少该字段时保持兼容，默认 4。

```sh
gw add /absolute/path/to/repository
gw remove /absolute/path/to/repository
gw exclude /absolute/path/to/excluded
gw unexclude /absolute/path/to/excluded
gw interval 6h
gw concurrency 4
gw autopush fast_forward
gw install
```

除应用调度外，配置命令不会触发检查或推送；取消排除也不会自动导入仓库。

## 终端界面

直接运行 `gw`，或显式运行 `gw tui`。至少需要 60 列 × 18 行的交互式终端；重定向输入输出时明确报错。进入界面只读取配置和已有报告，退出时恢复终端。

仓库列表每项固定一行，区分工作区、远程检查、无上游分支数和推送记录。默认按失败、需关注、未检查、正常排序；`Alt+O` 切换为路径顺序。宽屏显示选中仓库预览，完整失败原因可在详情中滚动查看。

- 底部输入框常驻，以边框、光标和弱化的占位提示区分可编辑区域；直接输入搜索仓库；文件面板中搜索文件。
- 输入 `/` 显示输入框上方的全宽命令面板，最多显示 8 条候选；高度不足时减少可见条数，其余通过方向键滚动。列表和输入框位置保持稳定。
- 搜索时方向键选择列表，`Enter` 详情或文件 diff；命令时方向键选择候选、`Tab` 补全、`Enter` 执行完整命令。
- `Esc` 清空输入；输入为空时返回上一页。`Ctrl+U` 清空光标前文字。
- `Ctrl+F` 循环切换全部、问题、无上游、推送；`Alt+O` 切换排序。
- `PgUp/PgDn` 翻页，`Tab` 切换设置，`F1` 帮助，`Ctrl+R` 刷新，`Ctrl+C` 退出。
- 普通字母、数字和空格用于输入，不再触发单字母快捷操作。
- 确认和执行期间输入框显示当前阶段，禁止普通输入；提交说明和配置输入复用同一区域。

顶部集中展示历史报告摘要和检查时间，仓库总数只在“全部”筛选中显示，调度与并发配置放在设置页。手动文件操作显示当前状态，仓库列表仍来自上次完整检查报告。

### 命令栏与单仓库操作

`/terminal` 在选中仓库目录打开系统默认终端（例如 Ghostty），供交互式 Git 操作或其他命令使用；输入 `/te` 后按 Tab 补全。macOS 通过 Launch Services 读取 `public.unix-executable` 的默认应用关联，再将目录作为独立路径参数交给该应用，不拼接 Shell 命令，也不固定使用 Apple Terminal。每次打开都重新读取系统关联，无需重启程序。未配置默认应用、目录不存在或启动失败时明确报错，不切换其他终端或目录。打开终端不改变历史检查报告，也不触发检查、提交或推送。

Linux 的 `/terminal` 通过 [`xdg-terminal-exec`](https://github.com/Vladimir-csp/xdg-terminal-exec) 的 `--dir` 参数打开首选终端。需另行安装该工具并通过 `xdg-terminals.list` 配置首选终端；这是 XDG 终端选择规范的提案实现，目录支持取决于所选终端的桌面条目。程序不会遍历猜测终端名称，也不会改用其他目录。启动确认仅表示启动器未立即失败，不代表用户已看到窗口；终端进程独立于检查任务存活。

命令候选明确标记当前仓库、当前视图或全局范围。仓库范围在输入命令前缀 `/` 时固定；命令栏不执行 Shell 命令。

| 命令 | 行为 |
| --- | --- |
| `/stage` | 打开工作区文件面板；在工作区中暂存所选文件 |
| `/unstage` | 打开暂存区文件面板；在暂存区中取消暂存所选文件 |
| `/diff` | 查看文件面板光标项的差异 |
| `/commit` | 预览暂存内容、输入提交说明，确认后提交 |
| `/push` | 获取当前分支对应远程，核对上游，确认后推送 |
| `/check all` | 确认后检查全部注册仓库，并按当前策略自动推送 |
| `/refresh` | 重新读取已有报告；文件面板中刷新本地状态 |
| `/settings`、`/help`、`/quit` | 设置、帮助、退出 |
| `/add`、`/remove`、`/exclude`、`/unexclude` | 管理注册列表和排除目录 |
| `/scan`、`/import` | 扫描预览、扫描并导入 |
| `/interval`、`/autopush`、`/concurrency`、`/apply` | 配置检查间隔、推送策略、并发数和应用调度 |

文件面板使用空输入时的 `Space` 或任意输入时的 `Insert` 勾选，`Ctrl+A` 全选或清空当前可见项，`Tab` 切换区域，`Enter` 查看 diff。普通文字按文件路径筛选，清除筛选后仍保留已勾选文件；操作作用于全部勾选项，没有勾选时作用于光标文件。暂存与取消暂存直接作用于显式选择的文件，取消暂存保留磁盘内容。同一文件的暂存和未暂存部分分开展示，Git 忽略的文件不列出。

文件名采用字面路径，支持空格、中文、换行和路径模式字符。未跟踪符号链接只显示链接目标；未跟踪大文件和过大的 diff 有明确显示限制。Git LFS、自定义外部 filter 和 `working-tree-encoding` 尚未注册到本地索引后端，涉及这些转换的暂存会明确拒绝，避免把未经转换的内容写入索引；需要使用 Git 显式处理。提交与推送仍走系统 Git，相关钩子正常执行。

提交只使用已经预览的暂存内容，不自动添加文件或推送，支持首次提交。推送只发布当前分支，独立于自动推送开关。没有上游、落后或分叉时明确报错，不自动配置、拉取、合并或强推。提交和推送统一通过 `/commit`、`/push` 进入。

确认前后会核验分支、HEAD、暂存内容和推送目标。提交钩子更改实际内容时明确提示核对实际提交。写入被取消时提示核对实际状态。单仓库操作与定时检查共享进程锁，操作日志保存在本机，不覆盖历史检查报告。

### 状态说明

- `✓` 正常，`!` 需关注，`×` 检查失败，`·` 未检查。
- 工作区 `*改动` 包括未提交、未跟踪和子模块改动；忽略文件不构成问题。
- 远程列只展示已经配置远程上游的本地分支结果；“待同步”表示存在领先或落后的提交。
- 无上游分支单独统计，不构成问题，也不会自动发布。
- 推送 `↑n !m` 为上次检查成功、失败的分支数，独立于提交数量。
- `?` 表示没有报告或检查未完成，该项未知。

检查失败的仓库不推测工作区和分支统计；此前已经完成的推送记录会保留。TLS 错误、上游引用不一致有简短中文分类，完整原始错误在预览和详情中显示。

交互参考 [mani](https://manicli.com/output/)，状态组织参考 [gita](https://github.com/nosarthur/gita)，功能定位仍为多仓库检查、受控推送与管理。

## 检查与并发

每轮读取一次显式配置，默认最多同时检查四组仓库。独立仓库并行；通过 libgit2 的 common directory 识别共享 Git 存储的工作区，同组按配置顺序串行检查。每个工作线程独立打开 libgit2 仓库对象，不跨线程共享可变仓库句柄。

检查顺序：

- 确认配置路径为仓库根目录。
- 系统 Git 执行 `fetch --all --no-recurse-submodules`。
- libgit2 读取工作区、子模块、HEAD、本地分支和上游，并比较提交图。
- 通过系统 Git 的 `ls-remote` 查询服务器引用，确认 fetch 后的跟踪引用与服务器一致。
- 按推送策略处理领先且不落后的已提交内容。
- 汇总结果，按配置列表顺序原子保存完整报告。

进度按实际完成顺序返回，报告顺序稳定。单个仓库失败不会中止其他仓库。取消会停止领取新任务、取消 Git/SSH/钩子进程组并等待工作线程退出，保留此前的完整报告；已经完成的远程写入不能撤销，需要核对实际远程状态。

报告保留 v3 数据格式，增加可选 `execution` 字段，记录并发上限和各仓库的总耗时、fetch、本地读取、远程核验、推送耗时，便于实际定位瓶颈。历史报告没有该字段仍能读取。

Git 命令默认超时 120 秒，SSH 禁止交互提示并限制连接等待。libgit2 的单次本地调用在返回边界响应取消，不会强行终止正在运行的 C 函数。

## 自动推送保证

`fast_forward` 只处理已配置远程上游、领先且不落后的本地分支。自动推送不会创建提交，也不会添加文件或发布无上游的实验分支。

- 拒绝未完成的 merge、rebase、cherry-pick 和未解决冲突。
- 验证检查地址与推送地址相同且地址唯一。
- 核对本地及服务器引用仍等于预览快照，并确认祖先关系。
- 固定提交 ID 和目标引用，显式禁用强推、镜像、标签跟随和递归子模块推送。
- 保留 pre-push 钩子；推送后再次查询服务器，确认目标提交已生效。
- 自动推送失败时发送“GW 自动推送失败”系统通知；通知失败也记入报告。

本地提交在核验后变化时不会被意外包含在本轮推送中；远程并发变化由普通推送规则拒绝非快进更新。工作区存在普通改动时，已提交的安全快照仍可发布，工作区问题继续单独统计。

## 扫描模块

`gw scan` 与 `gw import-tree` 使用独立扫描模块，不执行检查和推送。扫描范围外显式排除的目录不会进入；不沿目录符号链接递归。

未进入仓库前按目录寻找 `.git`。进入仓库后，由 libgit2 判断 `.gitignore`、`.git/info/exclude` 和全局排除规则，在进入被忽略目录前停止遍历。已跟踪路径和子模块 Git link 保留可达性；已跟踪目录下额外出现的忽略仓库不会因此暴露。

支持嵌套仓库、空嵌套仓库、子模块和关联工作区，进入嵌套仓库后使用其自身规则。扫描普通子目录时继承所属仓库规则；显式以仓库根目录为起点时使用该仓库自身规则。无效 `.git` 文件单独报告，损坏关联工作区记录为失败并停止向内盲目遍历。

```sh
gw scan /absolute/path/to/projects
gw import-tree /absolute/path/to/projects
```

## 查询与数据

```sh
gw status
gw status --json
gw report --problems
gw report --no-upstream
gw report --pushes
gw report --json
gw check
```

`status` 和 `report` 只读取已有配置和结果。`gw check` 是显式执行入口，会应用自动推送策略。退出码：0 成功且检查正常，1 存在需关注结果、扫描失败项或暂无报告，2 参数或执行错误，130 取消。

本机数据：

- `config.json`：显式配置。
- `var/latest.json`：最近一次完整检查报告。
- `var/import-report.json`：最近一次扫描报告。
- `var/manual-commit.json`、`var/manual-push.json`：最近一次已执行的单仓库操作记录。
- `var/check.lock`：跨进程排他锁；不会通过删除文件解锁。
- `var/launchd.stdout.log`、`var/launchd.stderr.log`：macOS 系统任务输出；Linux 输出保存在用户 journal。

配置、报告和日志含本机私有路径。JSON 数据原子写入，文件权限 0600，运行目录 0700。报告缺失与报告损坏分开处理，损坏数据明确报错。

## Workspace 与模块结构

项目使用 Cargo workspace，五个生产 crate 和一个仅供测试的辅助 crate：

| Crate | 职责 | 项目内运行时依赖 |
| --- | --- | --- |
| `app`（根目录） | CLI 命令解析、文本与 JSON 输出、程序入口 | `engine`、`tui`、`i18n`、`identity` |
| `engine` | 领域模型、本地 Git 能力、传输、应用服务、持久化和系统集成 | `i18n`、`identity` |
| `tui` | Ratatui 交互状态、命令栏、文件面板和视图 | `engine`、`i18n`、`identity` |
| `i18n` | 类型化中英文文案、系统语言检测与线程语言作用域 | `identity` |
| `identity` | 从 `application.env` 编译产品标识，供界面、通知和安装共用 | 无 |
| `test-support` | 共享临时本地 Git 仓库夹具，不发布 | 无；只作为其他 crate 的开发依赖 |

`engine` 不依赖 CLI 或 TUI，也不引入 Ratatui/Crossterm；`tui` 不依赖 CLI，也不直接依赖 `git2`，通过 engine 的文件快照、提交预览、推送预览和应用服务执行操作。CLI 和 TUI 各自负责输出格式化，避免界面适配器互相调用。

核心内部继续按能力拆分：

| 模块 | 职责 |
| --- | --- |
| `crates/engine/src/domain.rs` | 结果枚举、非空问题列表、兼容的 JSON 数据边界 |
| `crates/engine/src/local/` | libgit2 仓库、分支、状态、索引、差异和提交预览 |
| `crates/engine/src/transport.rs` | 固定系统 Git 网络与提交命令 |
| `crates/engine/src/operations/` | 提交和推送的预览、核验、执行 |
| `crates/engine/src/check.rs` | 单仓库检查流程 |
| `crates/engine/src/executor.rs` | 有界任务池、同组串行、进度汇聚 |
| `crates/engine/src/discovery.rs` | 忽略规则感知的仓库发现 |
| `crates/engine/src/service/` | 配置、快照、检查调度和单仓库应用服务 |
| `crates/engine/src/state.rs` | 原子持久化与进程锁 |
| `crates/engine/src/scheduler/`、`notify/`、`terminal/` | 各平台调度、桌面通知与终端启动 |
| `crates/engine/src/process.rs`、`cancel.rs` | 子进程收尾、超时与取消 |
| `crates/tui/src/input/`、`view/`、`task.rs` | 交互转换、绘制和异步任务收尾 |
| `src/cli.rs`、`src/main.rs` | CLI 适配和程序入口 |

依赖版本统一放在根目录的 `[workspace.dependencies]`；每个 crate 显式声明自己使用的依赖。互斥状态使用枚举表达，配置和历史 JSON 是明确的数据边界。

## 程序标识与更名

所有 crate 都设置 `publish = false`，内部名称按职责命名，不包含产品前缀。入口包及构建产物固定叫 `app`；公开命令名与内部包名相互独立。

产品标识只在根目录 `application.env` 定义：

| 字段 | 用途 |
| --- | --- |
| `APP_COMMAND` | 安装命令名、`dist/` 文件名、帮助和操作提示中的命令名 |
| `APP_DISPLAY_NAME` | 系统通知中的产品显示名 |
| `APP_DATA_DIRECTORY` | 各平台安装定位目录名 |
| `APP_SERVICE_LABEL` | 系统任务标识及定义文件名 |

该文件是共享的 `KEY=VALUE` 配置；值使用 ASCII 字母、数字、点、下划线和连字符，命令名以字母开头。不使用 Shell 表达式、空格或引号。`identity` 在编译时严格检查字段和取值，Shell 脚本与 Rust 读取同一份定义，不依赖额外运行时。

以后修改 `APP_COMMAND` 后重新构建、安装即可更换公开命令，无需改 Cargo 包名、依赖引用、帮助文案或安装脚本。数据目录和调度标签单独定义，避免仅修改命令名就丢失已有定位或调度。修改这些持久标识时需显式安排迁移；安装器不猜测旧路径。

项目内统一入口为 `./run.sh`，当前 `./gw` 仅作为兼容启动脚本转发到它。语言覆盖固定使用不含产品前缀的 `APP_LANG`，不随命令更名而改变。

## 开发与验证

```sh
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --locked --all-targets -- -D warnings
./build.sh
```

测试使用临时本地仓库和 bare 远程，不访问真实仓库、不访问网络、不发送系统通知、不安装用户系统任务。覆盖 Git 忽略规则、子模块、关联工作区、索引、diff、提交钩子、受控推送、并发上限、真实 fetch 重叠、取消与子进程收尾、旧数据兼容和 TUI 操作。

GitHub Actions 在 `macos-latest` 和 `ubuntu-24.04` 原生 runner 上执行格式检查、Clippy、全部测试和发布构建。Ubuntu 另验证真实 systemd 用户任务的安装、重复安装、延迟首轮、临时仓库检查和卸载；不使用 Docker。桌面窗口和通知显示仍需相应桌面会话手动验证。工作流在 push、pull request 或手动触发时执行，见 `.github/workflows/ci.yml`。

## 许可证

项目采用 MIT，见 `LICENSE`。依赖遵守各自许可证；libgit2 的链接例外允许应用采用独立许可证。公开源码与示例配置不得包含真实仓库列表、挂载目录、机器用户名或凭据。
