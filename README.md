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
  "max_concurrency": 4,
  "check_on_startup": true
}
```

仓库与排除目录以规范绝对路径保存，路径参数可使用 `~` 或 `~/…` 表示当前用户主目录。仓库列表显式维护，检查不会重新扫描发现目录。已排除目录下的仓库不能注册。

- `interval_seconds`：检查间隔，至少 60 秒。修改后运行 `gw install` 应用系统调度。
- `repositories`：仓库根目录列表，包括通过 `.git` 文件关联的工作区与子模块。
- `excluded_roots`：排除目录；其下的注册项会在排除操作时移除。
- `push_policy`：定时任务的自动推送策略；`disabled` 只检查，`fast_forward` 允许受控自动推送。
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

`check_on_startup` 控制每次启动 TUI 后是否后台检查全部注册仓库，默认 `true`，旧配置缺少该字段时也默认开启。通用设置中可编辑，或运行 `gw startup-check false` 关闭；修改只对下次进入 TUI 生效，不要求重新应用系统调度，也不立即检查。没有注册仓库或已有检查正在运行时不会重复发起。启动检查只执行 fetch 和状态比较，不应用定时任务的自动推送策略；`status`、`report` 和配置命令仍只读取或修改配置，不触发启动检查。

## 终端界面

直接运行 `gw`，或显式运行 `gw tui`。至少需要 60 列 × 18 行的交互式终端；重定向输入输出时明确报错。进入界面先显示已有结果，再按“启动时检查全部仓库”设置发起一次后台检查，默认开启；退出时取消本界面发起的检查并恢复终端。

仓库列表每项固定一行，展示工作区和同步状态；同步列合并远程检查与推送结果，失败及待同步优先于成功记录。无上游分支保留在详情与筛选中，不单独占用列表列。默认按失败、需关注、未检查、正常排序；`Alt+O` 切换为路径顺序。宽屏显示选中仓库预览，完整失败原因可在详情中滚动查看。

- 底部输入框常驻，以边框、光标和弱化的占位提示区分可编辑区域；直接输入搜索仓库；文件面板中搜索文件。
- 输入 `/` 显示输入框上方的全宽命令面板，最多显示 8 条候选；高度不足时减少可见条数，其余通过方向键滚动。列表和输入框位置保持稳定。
- 搜索时方向键选择列表，`Enter` 详情或文件 diff；命令时方向键选择候选、`Tab` 补全、`Enter` 执行选中命令。
- `Esc` 清空输入；输入为空时返回上一页。`Ctrl+U` 清空光标前文字。
- 输入框为空时，无修饰的 `→` 与 Enter 一样进入选中仓库、文件 diff 或设置项，`←` 与 Esc 一样返回上一级；输入框有文字时，左右键始终移动光标。确认界面中的左右键仍切换操作选项。
- `Ctrl+F` 循环切换全部、问题、无上游、推送；`Alt+O` 切换排序。
- 鼠标滚轮或触控板在主列表区域滚动，每次移动三项；方向键和翻页保留当前视口；键盘接管后忽略当前滚动序列剩余的同向惯性事件，避免选中项被拉回。
- `PgUp/PgDn` 翻页，`Tab` 循环切换仓库、通用设置和定时任务，`F1` 帮助，`Ctrl+R` 刷新，`Ctrl+C` 退出。
- 普通字母、数字和空格用于输入，不再触发单字母快捷操作。
- `F1` 或 `/help` 打开分组速查页，标题、快捷键、命令和说明使用不同样式；右侧滚动条显示当前位置，可用方向键和 PgUp/PgDn 浏览。
- 确认时底部显示横向操作选项，左右键或 Tab 切换、Enter 执行、Esc 返回；提交说明与完整参数命令返回编辑时保留。前台操作执行期间输入框显示当前阶段，禁止普通输入；后台检查不锁定输入框；提交说明复用底部输入区域，配置参数直接在命令栏填写。

顶部集中展示当前已知结果摘要和最近整轮检查时间，仓库总数只在“全部”筛选中显示，调度与并发配置分别放在任务管理页和通用设置页。手动文件操作显示当前状态，仓库列表合并历史报告与已完成的检查结果。

### 命令栏与单仓库操作

`/terminal` 在选中仓库目录打开系统默认终端（例如 Ghostty），供交互式 Git 操作或其他命令使用；输入 `/te` 后按 Tab 补全。macOS 通过 Launch Services 读取 `public.unix-executable` 的默认应用关联，再将目录作为独立路径参数交给该应用，不拼接 Shell 命令，也不固定使用 Apple Terminal。每次打开都重新读取系统关联，无需重启程序。未配置默认应用、目录不存在或启动失败时明确报错，不切换其他终端或目录。打开终端不改变历史检查报告，也不触发检查、提交或推送。

Linux 的 `/terminal` 通过 [`xdg-terminal-exec`](https://github.com/Vladimir-csp/xdg-terminal-exec) 的 `--dir` 参数打开首选终端。需另行安装该工具并通过 `xdg-terminals.list` 配置首选终端；这是 XDG 终端选择规范的提案实现，目录支持取决于所选终端的桌面条目。程序不会遍历猜测终端名称，也不会改用其他目录。启动确认仅表示启动器未立即失败，不代表用户已看到窗口；终端进程独立于检查任务存活。

命令候选明确标记当前仓库、当前视图或全局范围。仓库范围在输入命令前缀 `/` 时固定；命令栏不执行 Shell 命令。

| 命令 | 行为 |
| --- | --- |
| `/stage` | 打开工作区文件面板；在工作区中暂存所选文件 |
| `/unstage` | 打开暂存区文件面板；在暂存区中取消暂存所选文件 |
| `/diff` | 查看文件面板光标项的差异；子命令 `auto`、`split`、`unified` 切换布局 |
| `/commit` | 预览暂存内容、输入提交说明，确认后提交 |
| `/push` | 获取当前分支对应远程，核对上游，确认后推送 |
| `/check` | 立即在后台检查当前选中仓库，不推送 |
| `/cancel` | 取消后台检查，保留已完成的界面结果 |
| `/check all` | 立即在后台检查全部注册仓库，不推送 |
| `/refresh` | 重新读取已有报告；文件面板中刷新本地状态 |
| `/repo` | 仓库管理页；子命令 `add`、`remove`、`exclude`、`unexclude`、`scan`、`import` |
| `/settings` | 通用设置页；子命令 `concurrency` 配置检查并发数、`startup-check` 配置启动检查 |
| `/schedule` | 独立管理页，查看状态和配置、启停任务；保留 `interval`、`autopush`、`apply`、`enable`、`disable` 子命令 |
| `/help`、`/quit` | 帮助、退出 |

根命令候选只显示顶层入口，不平铺子配置；选中候选后按 Enter 直接进入，Tab 只补全命令名称。输入 `/schedule `、`/settings `、`/repo ` 或 `/check ` 后显示对应的下一层候选。例如 `/schedule interval` 修改执行间隔，`/repo remove` 移除选中仓库的注册。命令行 CLI 的 `gw interval` 等命令保持不变。

带参数的命令直接在常驻输入栏填写，不打开独立表单：

- `/repo add <path>`、`/repo exclude <path>`、`/repo unexclude <path>`、`/repo scan <path>`、`/repo import <path>`。
- `/schedule interval <duration>`，例如 `/schedule interval 6h`。
- `/schedule autopush <policy>`，取值 `disabled` 或 `fast_forward`。
- `/settings concurrency <count>`，范围 1 至 32。
- `/settings startup-check <true|false>`，控制启动时检查全部仓库，默认 `true`。

候选中的参数占位符使用亮青色斜体，与命令名称区分。选中命令按 Enter 或 Tab，只补全名称并追加空格；继续输入参数，Enter 校验并预览确认。参数缺失或错误时保留原输入。确认返回时保留整条命令和光标位置；管理页中的编辑操作也会在原页面下方填入完整命令及当前值。路径使用命令之后的整段文本，可包含空格；可整段使用匹配的单引号或双引号包裹，不执行 Shell、变量展开、通配展开或反斜线转义。`/repo remove` 仍作用于当前选中仓库，不需要路径参数。

TUI 和原生命令行的路径参数（包括 `--home`）均显式支持 `~` 和 `~/…`，使用当前进程 `HOME` 对应的用户主目录，与应用数据目录 `--home` 无关。例如 `/repo add ~/Proj`、`/repo scan "~/中文 目录"`。TUI 中单双引号包裹也会展开；输入栏补全后仍保留 `~/…`，确认预览与实际操作使用展开后的绝对路径，配置中不保存 `~`。仅展开开头独立的 `~`，路径中间的 `~`、`$HOME` 等仍为字面文本；不支持 `~user`。主目录缺失或无效时明确报错，不改用其他目录。

路径参数中按 `Tab` 只补全光标所在的一级目录：唯一匹配会补全并追加 `/`，多个匹配先补到公共前缀并显示最多 8 行目录候选。用上下键、Tab／Shift+Tab 选择，Enter 填入目录后继续编辑，再按 Enter 进入操作预览；Esc 先关闭候选。空格、中文和原有引号均按字面路径处理，补全不会插入 Shell 转义，光标后的路径部分保留。空路径从 `/` 开始；隐藏目录需先输入 `.` 才列出。只读当前一级目录，不递归、不检查仓库，也不执行 Git 操作；目录不可读或无匹配时明确提示并保留输入。

`/schedule` 为独立任务管理页，展示启用状态、检查进程、已保存间隔、自动推送策略以及调度配置是否已应用。上下键选择修改间隔、修改推送策略、应用配置和启用／停用任务，Enter 打开，Ctrl+R 刷新状态，Esc 返回。间隔保存后可能与系统任务仍在使用的间隔不同，页面会明确显示待应用；推送策略由定时进程每轮读取，下次执行即生效。应用配置保留任务当前的启停状态，不立即检查。

启用任务会安装或更新本机命令和系统任务，首轮仍等待配置间隔。停用仅停止后续调度，保留任务定义、配置、报告和命令；macOS 同时保存 launchd 的禁用覆盖项，避免下次登录后自动恢复；Linux 使用 `systemctl --user disable --now` 停用 timer。原生 CLI 也提供 `gw schedule enable`、`gw schedule disable`、`gw schedule apply`，它们与 TUI 使用相同的服务入口、归属校验和进程锁。正在检查时不能执行这些写操作，需等待检查结束或先取消手动检查。

文件面板使用空输入时的 `Space` 或任意输入时的 `Insert` 勾选，`Ctrl+A` 全选或清空当前可见项，`Tab` 切换区域，`Enter` 查看 diff。普通文字按文件路径筛选，清除筛选后仍保留已勾选文件；操作作用于全部勾选项，没有勾选时作用于光标文件。暂存与取消暂存直接作用于显式选择的文件，取消暂存保留磁盘内容。同一文件的暂存和未暂存部分分开展示，Git 忽略的文件不列出。

文件名采用字面路径，支持空格、中文、换行和路径模式字符。未跟踪符号链接只显示链接目标；未跟踪大文件和过大的 diff 有明确显示限制。Git LFS、自定义外部 filter 和 `working-tree-encoding` 尚未注册到本地索引后端，涉及这些转换的暂存会明确拒绝，避免把未经转换的内容写入索引；需要使用 Git 显式处理。提交与推送仍走系统 Git，相关钩子正常执行。

Diff 同时显示代码语法颜色和增删背景：新增行浅绿底、删除行浅红底，保留 `+`／`-` 标记、新旧行号及区块标题。语法由内置 Syntect 定义识别文件名、扩展名和首行，不依赖外部高亮命令、语法目录或运行时；使用纯 Rust 正则后端。旧版和新版源码分别从完整上下文解析，再将颜色映射到 diff 行号，跨行注释、字符串和省略区块不会混入另一侧状态。暂存 diff 比较 HEAD 与索引，工作区 diff 比较索引与磁盘内容。

读取和高亮在文件工作线程执行，可取消；当前 diff 缓存已着色逻辑行，滚动不会重新解析，窗口宽度或布局改变后只重新排版。源码读取每侧上限为 1 MiB，语法解析另限制单行 16 KiB、总计 50,000 行，patch 显示上限仍为 128 KiB。未知语言、超过解析限制、二进制或非普通文件会明确说明可用范围；有文本 patch 时保留差异颜色。读取期间源码与 patch 不一致会要求刷新，不套用错位的语法颜色。Tab 按 4 列展开，源码中的终端控制序列不会执行。

差异默认使用自动布局：内容区域至少 140 列、扣除行号后每侧代码区域至少 60 列时，显示左右对比；其余使用统一视图。左侧为旧版，右侧为新版，固定栏头标明 HEAD／暂存区或暂存区／工作区。修改块按顺序配对新增与删除行，缺少的一侧留空，长行按字符簇换行并补齐两边高度；同一滚动位置控制两侧。文件头和区块标题共用全宽行，不重复显示。缩放或切换布局时使用逻辑行与源码字节位置重新定位，重复缩放不会逐步漂移。

`/diff auto`、`/diff split`、`/diff unified` 支持 Tab 补全，立即切换当前视图，也应用于随后打开的文件；偏好仅在本次 TUI 会话保留，不修改配置或报告。切换不重新读取 Git、不重新高亮，也不需要确认。手动选择双栏后即使宽度不足自动阈值仍保持双栏；小到无法容纳行号及代码时明确提示扩大窗口或改回自动模式，并保留当前阅读位置。

提交只使用已经预览的暂存内容，不自动添加文件或推送，支持首次提交。推送只发布当前分支，独立于自动推送开关。没有上游、落后或分叉时明确报错，不自动配置、拉取、合并或强推。提交和推送统一通过 `/commit`、`/push` 进入。

确认前后会核验分支、HEAD、暂存内容和推送目标。提交钩子更改实际内容时明确提示核对实际提交。写入被取消时提示核对实际状态。单仓库操作与定时检查共享进程锁，操作日志保存在本机，不覆盖历史检查报告。

### 状态说明

- `✓` 正常，`!` 需关注，`×` 检查失败，`·` 未检查。
- 工作区 `*改动` 包括未提交、未跟踪和子模块改动；忽略文件不构成问题。
- 同步列合并远程检查与推送结果；“待同步”表示存在领先或落后的提交，失败和待同步优先于成功记录。
- 无上游分支单独统计，不构成问题，也不会自动发布。
- 同步列中的“已推送 N”和“推送失败 · N”按分支计数，与提交数量无关。
- `?` 表示没有可用的历史结果；检查尚未完成时，有旧结果的仓库仍保留旧状态。

检查失败的仓库不推测工作区和分支统计；此前已经完成的推送记录会保留。TLS 错误、上游引用不一致有简短中文分类，完整原始错误在预览和详情中显示。

交互参考 [mani](https://manicli.com/output/)，状态组织参考 [gita](https://github.com/nosarthur/gita)，功能定位仍为多仓库检查、受控推送与管理。

后台检查不占用界面的交互模式：搜索、列表导航、详情、帮助、设置浏览和只读文件面板仍可操作。正在检查的仓库显示旋转标记，已完成仓库立即更新结果；排队和未完成仓库继续显示已有结果，没有历史结果时显示未知。顶部显示检查进度与 `/cancel` 提示，检查完成不会清空搜索或跳离当前页面。

`/cancel` 请求取消并等待 Git 子进程收尾，期间仍可浏览；关闭 TUI 会取消并回收它启动的检查任务。已有整轮报告在检查成功结束前不会被覆盖，取消产生的错误也不会替换未完成仓库的旧状态。手动检查只执行 fetch 和本地快照比较，不自动推送；单仓库检查只处理目标注册仓库。检查期间提交、推送、暂存及配置修改受写锁保护；查看文件与差异仍可进行。

单仓库结果保存在独立的 `var/repository-checks.json`，重开 TUI 后仍会显示。TUI 按结果时间合并单仓库与整轮报告，较新的整轮结果覆盖旧的单仓库结果；全局检查时间和 `gw report` 始终表示最近一次完整批次。取消整轮检查后，已完成的逐仓库结果保留在当前界面中，未完成部分继续使用历史结果。

## 检查与并发

每轮读取一次显式配置，默认最多同时检查四组仓库。独立仓库并行；通过 libgit2 的 common directory 识别共享 Git 存储的工作区，同组按配置顺序串行检查。每个工作线程独立打开 libgit2 仓库对象，不跨线程共享可变仓库句柄。

检查顺序：

- 确认配置路径为仓库根目录。
- 校验上游分支的 fetch refspec：支持通配、精确及负向规则、自定义远程跟踪命名空间；未覆盖、被排除、映射不唯一或远程被 `skipFetchAll` 跳过时明确报错。
- 按远程执行一次系统 Git：`fetch --prune --no-prune-tags --no-tags --no-recurse-submodules --refmap= -- <remote> <远程跟踪 refspec...> <负向 refspec...>`。仅传入目标位于 `refs/remotes/` 的配置映射，不抓取或清理标签、本地分支等其他目标。清理范围重叠时在 fetch 前拒绝执行。
- libgit2 读取工作区、子模块、HEAD、本地分支和上游，并比较提交图。
- 固定 fetch 后的本地跟踪引用快照进行比较，普通检查不执行 `ls-remote`。通配规则下已删除的上游在 prune 后报告为上游不存在，仍属于已配置上游；精确 refspec 指向已删除分支时，Git 会拒绝 fetch，程序保留其失败原因，不使用旧引用判定同步。
- 仅定时任务按推送策略处理领先且不落后的已提交内容。
- 汇总结果，按配置列表顺序原子保存完整报告。

进度按实际完成顺序返回，报告顺序稳定。单个仓库失败不会中止其他仓库。取消会停止领取新任务、取消 Git/SSH/钩子进程组并等待工作线程退出，保留此前的完整报告；已经完成的远程写入不能撤销，需要核对实际远程状态。

报告保留 v3 数据格式，增加可选 `execution` 字段，记录并发上限和各仓库的总耗时、fetch、本地读取、远程核验、推送耗时，便于实际定位瓶颈。普通检查不再单独查询远程，保留兼容字段 `verify_ms` 为 0；推送前后的远程核验包含在 `push_ms` 中。历史报告没有该字段仍能读取。

Git 命令默认超时 120 秒，SSH 禁止交互提示并限制连接等待。libgit2 的单次本地调用在返回边界响应取消，不会强行终止正在运行的 C 函数。

## 自动推送保证

自动推送配置仅用于定时任务入口 `gw scheduled-check`；手动 `gw check [路径]`、`/check` 和 `/check all` 均不推送，也无需确认。

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
gw check /absolute/path/to/repository  # 只检查一个已注册仓库
```

`status` 和 `report` 只读取已有配置和结果。`gw check [路径]` 是手动检查入口，不执行推送。系统调度使用独立的 `gw scheduled-check` 入口，才应用自动推送配置。退出码：0 成功且检查正常，1 存在需关注结果、扫描失败项或暂无报告，2 参数或执行错误，130 取消。

本机数据：

- `config.json`：显式配置。
- `var/latest.json`：最近一次完整检查报告。
- `var/repository-checks.json`：独立的单仓库检查报告。
- `var/import-report.json`：最近一次扫描报告。
- `var/manual-commit.json`、`var/manual-push.json`：最近一次已执行的单仓库操作记录。
- `var/check.lock`：跨进程排他锁；不会通过删除文件解锁。
- `var/launchd.stdout.log`、`var/launchd.stderr.log`：macOS 系统任务输出；Linux 输出保存在用户 journal。

配置、报告和日志含本机私有路径。JSON 数据原子写入，文件权限 0600，运行目录 0700。报告缺失与报告损坏分开处理，损坏数据明确报错。

## Workspace 与模块结构

项目使用 Cargo workspace，六个生产 crate 和一个仅供测试的辅助 crate：

| Crate | 职责 | 项目内运行时依赖 |
| --- | --- | --- |
| `app`（根目录） | CLI 命令解析、文本与 JSON 输出、程序入口 | `engine`、`tui`、`i18n`、`identity` |
| `engine` | 领域模型、本地 Git 能力、传输、应用服务、持久化和系统集成 | `i18n`、`identity` |
| `tui` | Ratatui 交互状态、命令栏、文件面板和视图 | `engine`、`highlight`、`i18n`、`identity` |
| `highlight` | 有界源码语法解析、语言识别及与界面无关的颜色区间 | 无 |
| `i18n` | 类型化中英文文案、系统语言检测与线程语言作用域 | `identity` |
| `identity` | 从 `application.env` 编译产品标识，供界面、通知和安装共用 | 无 |
| `test-support` | 共享临时本地 Git 仓库夹具，不发布 | 无；只作为其他 crate 的开发依赖 |

`engine` 不依赖 CLI 或 TUI，也不引入 Ratatui/Crossterm；`tui` 不依赖 CLI，也不直接依赖 `git2`，通过 engine 的文件快照、提交预览、推送预览和应用服务执行操作。CLI 和 TUI 各自负责输出格式化，避免界面适配器互相调用。

`highlight` 不依赖 engine、Ratatui 或应用服务，输入源码、路径、所需行号及取消检查回调，输出明确的语法可用状态和颜色区间。它只保留所需行的结果，但解析这些行之前的上下文；语法定义与主题在进程中只加载一次。engine 的结构化 patch 不包含颜色；TUI 将高亮结果与 diff 背景、行号和终端安全渲染组合，后续替换语法后端无需修改 Git 读取逻辑。

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
| `crates/tui/src/input/`、`view/`、`task.rs`、`checks.rs` | 交互转换、绘制、前台任务和独立后台检查 |
| `crates/tui/src/diff/` | 逻辑行高亮缓存、统一／双栏排版、字符簇换行和阅读位置映射 |
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
