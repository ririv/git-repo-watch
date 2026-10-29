"""Attach a real submodule with a local remote and independent worktree changes."""
from git_repo import Clock, Git
from seed import Seed


MODULE_PATH = "modules/演练 库"


def attach(seed: Seed):
    root = seed.repo.path.parent
    clock = Clock()
    peer = Git(root / "submodule-peer", clock)
    peer.path.mkdir()
    peer.run("init", "-b", "main")
    peer.configure()
    peer.write("README.md", "# 示例子模块\n\n用于演练子模块提交、引用更新和未提交内容。\n")
    peer.write("src/library.ts", 'export const greeting = "原始问候";\n')
    peer.commit("初始化示例子模块")
    peer.run("tag", "example/submodule-base")

    remote = root / "submodule-remote.git"
    remote.mkdir()
    bare = Git(remote, clock)
    bare.run("init", "--bare", "-b", "main")
    bare.configure()
    peer.run("remote", "add", "origin", str(remote))
    peer.run("push", "--all", "origin")
    peer.run("push", "--tags", "origin")
    peer.run("branch", "--set-upstream-to=origin/main", "main")

    # 相对地址按父仓库的 origin 解析，不把输出目录写入提交历史。
    seed.repo.run("submodule", "add", "--name", "example-library", "../submodule-remote.git", MODULE_PATH)
    seed.repo.commit("登记示例库子模块")
    seed.repo.run("tag", "example/submodule")
    seed.repo.run("push", "origin", "main", "--tags")
    seed.peer.run("fetch", "origin")
    seed.peer.run("merge", "--ff-only", "origin/main")
    seed.peer.run("submodule", "update", "--init", "--recursive")
    Git(seed.peer.path / MODULE_PATH, clock).configure()

    child = Git(seed.repo.path / MODULE_PATH, clock)
    child.configure()
    child.run("branch", "--set-upstream-to=origin/main", "main")
    child.write("src/version.ts", 'export const version = "1.1.0";\n')
    child.commit("子模块新增本地版本")
    child.run("tag", "example/submodule-local")
    child.write("src/library.ts", 'export const greeting = "已暂存的问候";\n')
    child.run("add", "--", "src/library.ts")
    child.write("src/library.ts", 'export const greeting = "继续编辑的问候";\n')
    child.write("docs/未跟踪 说明.md", "# 子模块内部尚未跟踪的说明\n")
    seed.submodules = (Seed(child, peer, remote),)
