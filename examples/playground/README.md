# Git 演练仓库

用于手动体验 GW、复现问题和录制演示。生成器只需要 Python 3.9+ 与 PATH 中的 Git，使用标准库，无额外 Python 依赖。正式使用 GW 仍无需 Python。

## 创建并打开

在项目根目录执行：

```sh
python3 examples/playground/create.py
```

默认生成 `work/playground/workspace/`，程序会打印可直接执行的启动命令。也可使用：

```sh
PLAYGROUND_ROOT="$PWD/work/playground/workspace"
cd "$PLAYGROUND_ROOT/repo"
gw --home "$PLAYGROUND_ROOT/home"
```

从独立仓库列表进入：

```sh
gw --home "$PLAYGROUND_ROOT/home" repos
```

如需在列表中查看保存的检查结果，可手动检查这个示例：

```sh
gw --home "$PLAYGROUND_ROOT/home" check "$PLAYGROUND_ROOT/repo"
```

工作区有变更或分支有待同步提交时，检查返回码为 `1`，表示需关注。

## 目录组织

| 路径 | 内容 |
| --- | --- |
| `examples/playground/` | 纳入项目版本管理的生成脚本、素材、说明和测试 |
| `work/playground/<场景>/repo/` | 真正用于操作的 Git 仓库 |
| `work/playground/<场景>/remote.git/` | 本地 bare 远程，接收示例仓库的推送 |
| `work/playground/<场景>/peer/` | 第二个工作副本，用于制造远程更新 |
| `work/playground/<场景>/home/` | 独立的 GW 配置与运行数据 |
| `work/playground/<场景>/manifest.json` | 场景、初始 HEAD、标签提交 ID 与工作区状态 |
| `work/playground/submodule/submodule-remote.git/` | 子模块的本地 bare 远程 |
| `work/playground/submodule/submodule-peer/` | 子模块的第二个工作副本 |

`work/` 已在项目忽略规则中。生成脚本隔离调用者的 Git 环境和系统／全局配置，采用固定示例身份与提交时间，并关闭签名与 hooks。生成的仓库只允许本地 file 协议。独立 GW 配置关闭 AI，定时流程仅含本地状态读取；定时任务保持未安装。

## 场景

```sh
python3 examples/playground/create.py --case clean
python3 examples/playground/create.py --case cherry-pick-conflict
python3 examples/playground/create.py --case submodule
python3 examples/playground/create.py --case cherry-pick-batch
python3 examples/playground/create.py --case diff-metadata
python3 examples/playground/create.py --case stash-preview
```

| `--case` | 初始状态与用途 |
| --- | --- |
| `workspace` | 默认场景：同一文件既已暂存又有继续编辑，另有重命名、删除、中文及空格路径、未跟踪文件、忽略文件和 stash |
| `stash-preview` | 一份含原暂存、原未暂存、未跟踪文字／图片／二进制内容的 stash；当前索引与工作区另有不同内容，用于核对预览来源 |
| `diff-metadata` | 同一仓库包含普通修改、可执行权限、符号链接类型变化、增删、重命名、复制、二进制内容和末尾换行对照；改动均已暂存并另存历史标签 |
| `clean` | 干净且与上游同步，适合从历史中手动执行 cherry-pick、revert 和普通提交 |
| `ahead` | 当前分支 ↑1 ↓0，适合推送和同步确认 |
| `behind` | 当前分支 ↑0 ↓1，适合快进拉取和同步 |
| `diverged` | 当前分支 ↑1 ↓1，适合验证分叉提示 |
| `cherry-pick-conflict` | 两个文件共四个真实冲突块，另有自动合并的非冲突修改，可测试选块、采用、撤销、重置、暂存和继续 |
| `revert-conflict` | 已停在真实 revert 冲突，可测试撤销的恢复流程 |
| `no-remote` | 未配置远程，适合检查正常状态和同步提示 |
| `cherry-pick-batch` | 干净的 main 与未合入的 feature/batch；三个连续提交包含先新增再修改同一文件，可测试批量顺序 |
| `submodule` | 真实子模块：父仓库记录旧提交，子模块领先自身远程一个提交，内部同时包含已暂存、未暂存和未跟踪文件 |

每个场景都有分支 `main`、`feature/ui`、`feature/fix`，以及以下固定标签：

- `example/base`：共同起点。
- `example/cherry-pick`：待拣选的问候修复，与默认 main 中的变更互不冲突。
- `example/revert`：新增实验功能的提交，可在干净场景撤销。
- `example/merge`：包含两个父节点的合并，可检查历史连线和主线父节点选择。

仓库内含 TypeScript、Markdown、SVG、PNG 和紧凑目录链。README 的相对链接可用于检查文件跳转。

## 批量拣选演练

`cherry-pick-batch` 的 `feature/batch` 有三条提交，分别带 `example/batch-1`、`example/batch-2`、`example/batch-3` 标签，均未合入当前 `main`。在历史图执行 `/repo log select` 进入选择模式，用 `Space` 或 `Shift+↑↓` 勾选三条“批量演练”提交后按 `Enter` 打开操作菜单，选择“拣选所选提交”后预览，预览将按 1、2、3 排列；第二条修改第一条新增的文件，可验证提交依赖和实际执行顺序。

## 内置冲突解决演练

```sh
python3 examples/playground/create.py --case cherry-pick-conflict
```

沿用同一个演练仓库，不需要另外创建或登记仓库。目录已存在时指定新的 `--directory`，生成器不会覆盖已有演练。按生成器打印的命令启动 `gw`，执行 `/repo diff`。

- `src/config.ts` 有一个问候文案冲突。
- `src/合并演练.ts` 有三个分开的冲突块，分别修改标题、主题和重试次数。`localOnly` 与 `incomingOnly` 是两侧各自修改、由 Git 自动合并的内容，应在采用或重置后继续保留。
- `example/conflict` 是本次正在拣选的提交；基础、左、右均保存在真实 Git 索引阶段中。

进入冲突文件后默认打开底部“冲突处理”页。↑↓ 选择冲突块，←→ 选择左（当前）、右（传入）或双方，Enter 采用。采用不会自动暂存，最后一块处理完成后自动选中“暂存”。暂存当前文件后继续处理另一文件；全部冲突暂存后，底部转到 Git 流程的“继续”，确认后完成原来的拣选。

“撤销”恢复最近一次采用或重置。“重置”会先显示确认页，默认取消；确认后从基础、左、右重建当前文件全部冲突块，同时保留自动合并的内容。

Shift+Tab 在输入、冲突处理和 Git 流程间循环，右上角显示页码，切换不改变布局高度。Tab 仍只负责补全。场景改变后选择合适的默认页，同一场景中手动选择的页与搜索／命令草稿会保留。所有操作均针对当前演练仓库。

## Stash 内容预览演练

```sh
python3 examples/playground/create.py --case stash-preview
```

按生成器打印的命令启动，执行 `/repo stash`，选择“预览演练”并按 Enter。宽屏下应看到顶部说明、左侧文件与右侧连续差异；选择 `src/config.ts` 时预览定位到该文件，Enter 打开完整 diff，Esc 返回原选择。切窄窗口后，保留说明与文件列表，通过 Enter 查看差异。

详情中执行 `/scope` 打开范围菜单，可选 `all`、`staged`、`unstaged`；也可使用完整命令 `/repo stash scope staged`。`src/config.ts` 应分别显示基准到“保存时工作区内容”、基准到“保存时暂存内容”、暂存到工作区的差异，任何范围都不应显示“当前索引内容”或“当前工作区内容”。

- `stash/暂存后改回.ts`：最终工作区与基准相同，在 `all` 下没有差异；`staged`、`unstaged` 可分别查看暂存和改回的内容。
- `stash/中文 新文件.md`：未跟踪文件，应出现在 `all`、`unstaged` 下；可搜索中文或使用定位模式逐项查找。
- `stash/未跟踪图片.png`：Enter 进入图片预览。
- `stash/二进制.dat`：显示二进制提示。
- `docs/旧名称.md → docs/新名称.md` 和 `docs/待删除.md`：查看重命名与删除。

查看前后可以执行 `git status --short` 与 `git stash list`，结果应相同。该场景使用独立的 GW 配置，不启用定时任务。

## 文件头元信息演练

`diff-metadata` 的变更均在暂存区：进入 `/repo diff` 后查看已暂存区域中 `元信息/` 下的文件。普通修改直接显示变更块，文件模式变化显示摘要；`/repo diff header show` 展开原始对象 ID、模式和路径，`/repo diff header hide` 收起并恢复阅读位置；不填参数时打开选项菜单。复制差异保留完整 Git 补丁。

- `01-普通修改.ts`：验证 `diff --git`、`index`、`---/+++` 默认隐藏。
- `02-可执行权限.sh`：仅增加可执行标记，文件类型仍为普通文件，内容未改。
- `03-文件类型.txt`：普通文件改为指向 `01-普通修改.ts` 的符号链接。
- `04-待删除.md`、`07-新增文件.md`：删除和新增。
- `05-重命名后.md`：重命名并修改标题，相似度低于 100%。
- `06-复制目标.md`：复制原内容，来源文件同时继续编辑；工作区列表按 Git 状态展示，历史提交可查看复制关系。
- `08-二进制.dat`：保留二进制差异提示。

历史图中的 `example/metadata-changes` 标记同一份已提交改动；`example/metadata-base` 是比较基准。可检查连续预览和 Enter 打开的单文件预览是否一致。当前 `main` 停在基准，与本地远程同步，已暂存内容可用于继续手动演练。

## 末尾换行演练

末尾换行样例与上述元信息样例放在同一个 `diff-metadata` 仓库中，改动全部位于暂存区。执行 `/repo diff`，查看 `末尾换行/` 下五个文件末行下方的独立红色 `⊖` 标记：

| 文件 | 比较内容 | 标记位置 |
| --- | --- | --- |
| `01-补上末尾换行.txt` | 文字相同，只补上换行 | 旧版末行下方 |
| `02-移除末尾换行.txt` | 文字相同，只移除换行 | 新版末行下方 |
| `03-两侧都无换行.txt` | 修改文字，两侧都没有末尾换行 | 旧版、新版末行下方各一处 |
| `04-新增且无换行.txt` | 新增一个没有末尾换行的文件 | 新版末行下方 |
| `05-正常换行对照.txt` | 修改文字，两侧都有末尾换行 | 无提示 |

可切换 `/repo diff layout unified` 与 `/repo diff layout split` 对比。`example/newline-changes` 标签记录同一份历史改动；进入历史详情可查看连续预览。当前 `main` 停在 `example/newline-base`，与本地远程同步。

## 子模块演练

`submodule` 场景在父仓库中登记 `modules/演练 库/`，使用真实 `.gitmodules`、gitlink 和 `.git` 文件。子模块的本地远程与第二个工作副本同样留在该场景目录；`.gitmodules` 使用相对远程地址，因此同一场景在不同输出目录中生成相同提交历史。

演练专用的 GW 配置同时登记父仓库和子模块。从仓库列表分别进入：父仓库用于观察子模块引用变化，子模块用于查看内部文件和执行暂存、提交、推送。初始状态下，父仓库与远程同步，子模块有一个尚未推送的本地提交；`src/library.ts` 同时存在已暂存与继续编辑的内容，`docs/未跟踪 说明.md` 尚未跟踪。

父仓库的 `example/submodule` 标签标记登记子模块的提交；子模块中的 `example/submodule-base` 标记父仓库引用的版本，`example/submodule-local` 标记新增的本地提交。父仓库的第二个工作副本已初始化到干净基准，便于对比。`manifest.json` 另存子模块路径、当前提交、父仓库记录的提交及初始工作区状态。

## 保留与重建

创建时拒绝覆盖已有目录、文件和符号链接。需要全新数据时指定另一个目录，已有演练现场可以保留：

```sh
python3 examples/playground/create.py --case clean --directory "$PWD/work/playground/clean-2"
```

创建失败时保留现场，下一次使用新目录。所有生成内容都可以丢弃；运行时配置和数据库随对应的 `home/` 一起保存。

## 验证

```sh
python3 -m unittest discover -s examples/playground -p 'test_*.py' -v
```

测试使用临时目录，验证全部场景的真实 Git 状态、冲突恢复、可重建的历史、中文与空格路径、已有内容保护，以及 Git 环境隔离。子模块场景另验证本地远程可递归克隆并恢复干净基准。CI 在 macOS 和 Linux 上运行同一套检查。

## 上传方式

`examples/playground/` 随主项目上传，让其他开发者按需生成相同场景；生成的 `repo/`、本地 bare 远程、第二个工作副本、`home/` 和数据库留在 `work/`。主项目不将生成结果登记为子模块；`submodule` 场景中的子模块关系只存在于生成的演练仓库。

本地 bare 远程已足以演练 fetch、pull、push 和同步。以后需要验证 GitHub 认证、托管服务权限或 PR 时，再建立独立、可丢弃的私有演练远程，并只放合成数据。
