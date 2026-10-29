"""Apply one explicit manual-testing scenario to a fresh seed."""
from enum import Enum
from seed import Seed
from submodules import attach


class Case(Enum):
    WORKSPACE = "workspace"
    CLEAN = "clean"
    AHEAD = "ahead"
    BEHIND = "behind"
    DIVERGED = "diverged"
    CHERRY_PICK_CONFLICT = "cherry-pick-conflict"
    REVERT_CONFLICT = "revert-conflict"
    NO_REMOTE = "no-remote"
    SUBMODULE = "submodule"
    CHERRY_PICK_BATCH = "cherry-pick-batch"

    def __str__(self):
        return self.value


def workspace(seed: Seed):
    repo = seed.repo
    repo.write("README.md", (repo.path / "README.md").read_text(encoding="utf-8") + "\n待恢复的 stash 内容。\n")
    repo.run("stash", "push", "-m", "演示：尚未完成的说明", date=repo.clock.next())
    repo.write("src/config.ts", 'export const greeting = "已暂存的问候";\n')
    repo.run("add", "--", "src/config.ts")
    repo.write("src/config.ts", 'export const greeting = "继续编辑的问候";\n')
    repo.run("mv", "--", "docs/旧名称.md", "docs/新名称.md")
    (repo.path / "docs/待删除.md").unlink()
    repo.write("docs/新建 说明.md", "# 尚未跟踪的中文文件\n")
    repo.write("samples/compact/one-child/说明.txt", "单一目录链折叠测试。\n")
    repo.write("cache/ignored.tmp", "忽略文件不出现在工作区变更中。\n")


def clean(seed: Seed):
    # 种子本身就是与上游同步的干净仓库。
    return None


def ahead(seed: Seed):
    seed.repo.write("docs/local.md", "# 只在本地的提交\n")
    seed.repo.commit("本地新增说明")


def behind(seed: Seed):
    seed.peer.write("docs/remote.md", "# 只在远程的提交\n")
    seed.peer.commit("远程新增说明")
    seed.peer.run("push", "origin", "main")
    seed.repo.run("fetch", "origin")


def diverged(seed: Seed):
    ahead(seed)
    behind(seed)


def cherry_pick_conflict(seed: Seed):
    seed.repo.write("src/config.ts", 'export const greeting = "当前分支的问候";\n')
    seed.repo.commit("当前分支修改问候文案")
    seed.repo.conflict("cherry-pick", "example/cherry-pick")


def revert_conflict(seed: Seed):
    seed.repo.write("src/revert-me.ts", 'export const experimental = "已经继续开发";\n')
    seed.repo.commit("继续修改实验功能")
    seed.repo.conflict("revert", "example/revert")


def no_remote(seed: Seed):
    seed.repo.run("remote", "remove", "origin")


def cherry_pick_batch(seed: Seed):
    repo = seed.repo
    repo.run("checkout", "-b", "feature/batch", "example/base")
    for number, (name, content, message) in enumerate([
        ("src/batch.ts", 'export const batch = "first";\n', "批量演练：增加基础功能"),
        ("src/batch.ts", 'export const batch = "second";\n', "批量演练：调整基础功能"),
        ("docs/batch.md", "# 批量拣选演练\n", "批量演练：补充使用说明"),
    ], start=1):
        repo.write(name, content)
        repo.commit(message)
        repo.run("tag", f"example/batch-{number}")
    repo.run("push", "origin", "feature/batch", "--tags")
    repo.run("checkout", "main")


APPLY = {
    Case.WORKSPACE: workspace, Case.CLEAN: clean, Case.AHEAD: ahead,
    Case.BEHIND: behind, Case.DIVERGED: diverged,
    Case.CHERRY_PICK_CONFLICT: cherry_pick_conflict,
    Case.REVERT_CONFLICT: revert_conflict, Case.NO_REMOTE: no_remote,
    Case.SUBMODULE: attach,
    Case.CHERRY_PICK_BATCH: cherry_pick_batch,
}
