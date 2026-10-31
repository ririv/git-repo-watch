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

`work/` 已在项目忽略规则中。生成脚本隔离调用者的 Git 环境和系统／全局配置，采用固定示例身份与提交时间，并关闭签名与 hooks。生成的仓库只允许本地 file 协议。独立 GW 配置关闭启动检查和 AI，启动与定时流程仅含本地状态读取；定时任务保持未安装。

## 场景

```sh
python3 examples/playground/create.py --case clean
python3 examples/playground/create.py --case cherry-pick-conflict
python3 examples/playground/create.py --case submodule
python3 examples/playground/create.py --case cherry-pick-batch
python3 examples/playground/create.py --case diff-metadata
```

| `--case` | 初始状态与用途 |
| --- | --- |
| `workspace` | 默认场景：同一文件既已暂存又有继续编辑，另有重命名、删除、中文及空格路径、未跟踪文件、忽略文件和 stash |
| `diff-metadata` | 已暂存的普通修改、可执行权限、符号链接类型变化、增删、重命名、复制及二进制内容；同一份改动另存历史标签 |
| `clean` | 干净且与上游同步，适合从历史中手动执行 cherry-pick、revert 和普通提交 |
| `ahead` | 当前分支 ↑1 ↓0，适合推送和同步确认 |
| `behind` | 当前分支 ↑0 ↓1，适合快进拉取和同步 |
| `diverged` | 当前分支 ↑1 ↓1，适合验证分叉提示 |
| `cherry-pick-conflict` | 已停在真实 cherry-pick 冲突，可测试解决、暂存、继续、跳过和中止 |
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

## 文件头元信息演练

`diff-metadata` 的变更均在暂存区：进入 `/repo diff` 后查看已暂存区域中 `元信息/` 下的文件。普通修改直接显示变更块，文件模式变化显示摘要；`/repo diff header` 展开原始对象 ID、模式和路径，重复执行收起并恢复阅读位置。复制差异保留完整 Git 补丁。

- `01-普通修改.ts`：验证 `diff --git`、`index`、`---/+++` 默认隐藏。
- `02-可执行权限.sh`：仅增加可执行标记，文件类型仍为普通文件，内容未改。
- `03-文件类型.txt`：普通文件改为指向 `01-普通修改.ts` 的符号链接。
- `04-待删除.md`、`07-新增文件.md`：删除和新增。
- `05-重命名后.md`：重命名并修改标题，相似度低于 100%。
- `06-复制目标.md`：复制原内容，来源文件同时继续编辑；工作区列表按 Git 状态展示，历史提交可查看复制关系。
- `08-二进制.dat`：保留二进制差异提示。

历史图中的 `example/metadata-changes` 标记同一份已提交改动；`example/metadata-base` 是比较基准。可检查连续预览和 Enter 打开的单文件预览是否一致。当前 `main` 停在基准，与本地远程同步，已暂存内容可用于继续手动演练。

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

测试使用临时目录，验证十一种真实 Git 状态、冲突恢复、可重建的历史、中文与空格路径、已有内容保护，以及 Git 环境隔离。子模块场景另验证本地远程可递归克隆并恢复干净基准。CI 在 macOS 和 Linux 上运行同一套检查。

## 上传方式

`examples/playground/` 随主项目上传，让其他开发者按需生成相同场景；生成的 `repo/`、本地 bare 远程、第二个工作副本、`home/` 和数据库留在 `work/`。主项目不将生成结果登记为子模块；`submodule` 场景中的子模块关系只存在于生成的演练仓库。

本地 bare 远程已足以演练 fetch、pull、push 和同步。以后需要验证 GitHub 认证、托管服务权限或 PR 时，再建立独立、可丢弃的私有演练远程，并只放合成数据。
