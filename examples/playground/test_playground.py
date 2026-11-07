"""Verify real Git states and isolation of the manual playground."""
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from create import generate
from git_repo import Clock, Git
from scenarios import Case
from submodules import MODULE_PATH


class PlaygroundTests(unittest.TestCase):
    def test_scenarios_have_the_advertised_git_state(self):
        with tempfile.TemporaryDirectory() as temporary:
            for case in Case:
                with self.subTest(case=case.value):
                    root = generate(Path(temporary) / ("中文 空格 " + case.value), case)
                    repo = Git(root / "repo", Clock())
                    manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
                    self.assertEqual(repo.run("branch", "--show-current"), "main")
                    self.assertEqual(len(repo.run("rev-list", "--parents", "-n", "1", "example/merge").split()), 3)
                    self.assertEqual(manifest["case"], case.value)
                    self.assertEqual((repo.path / "媒体/colors.png").read_bytes()[:8], b"\x89PNG\r\n\x1a\n")
                    self.assertTrue((repo.path / "docs/带 空格的说明.md").is_file())
                    if case == Case.WORKSPACE:
                        status = repo.run("-c", "core.quotePath=false", "status", "--porcelain=v1")
                        self.assertIn("MM src/config.ts", status)
                        self.assertIn("docs/旧名称.md -> docs/新名称.md", status)
                        self.assertIn(" D docs/待删除.md", status)
                        self.assertIn("docs/新建 说明.md", repo.run("ls-files", "--others", "--exclude-standard", "-z").split("\0"))
                        self.assertTrue(repo.run("stash", "list"))
                        self.assertEqual(repo.run("check-ignore", "cache/ignored.tmp"), "cache/ignored.tmp")
                    elif case == Case.CHERRY_PICK_CONFLICT:
                        self.assertTrue(repo.run("ls-files", "--unmerged"))
                        self.assertEqual(repo.run("rev-parse", "CHERRY_PICK_HEAD"), manifest["refs"]["example/cherry-pick"])
                        repo.run("cherry-pick", "--abort")
                        self.assertEqual(repo.run("status", "--porcelain=v1"), "")
                    elif case == Case.REVERT_CONFLICT:
                        self.assertTrue(repo.run("ls-files", "--unmerged"))
                        self.assertEqual(repo.run("rev-parse", "REVERT_HEAD"), manifest["refs"]["example/revert"])
                        repo.run("rm", "--force", "--", "src/revert-me.ts")
                        repo.run("revert", "--continue", date=repo.clock.next())
                        self.assertEqual(repo.run("status", "--porcelain=v1"), "")
                        self.assertFalse((repo.path / "src/revert-me.ts").exists())
                    elif case == Case.NO_REMOTE:
                        self.assertEqual(repo.run("remote"), "")
                        self.assertEqual(repo.run("status", "--porcelain=v1"), "")
                    elif case == Case.SUBMODULE:
                        module = Git(repo.path / MODULE_PATH, Clock())
                        self.assertTrue((module.path / ".git").is_file())
                        self.assertEqual(module.run("rev-parse", "--show-superproject-working-tree"), str(repo.path))
                        mode, recorded = repo.run("ls-files", "--stage", "--", MODULE_PATH).split()[:2]
                        self.assertEqual(mode, "160000")
                        self.assertEqual(recorded, manifest["submodules"][0]["recorded_head"])
                        self.assertNotEqual(module.run("rev-parse", "HEAD"), recorded)
                        self.assertEqual(module.run("rev-list", "--left-right", "--count", "HEAD...@{upstream}"), "1\t0")
                        self.assertEqual(repo.run("rev-list", "--left-right", "--count", "HEAD...@{upstream}"), "0\t0")
                        self.assertIn("MM src/library.ts", module.run("status", "--porcelain=v1"))
                        self.assertIn("原始问候", module.run("show", "HEAD:src/library.ts"))
                        self.assertIn("已暂存的问候", module.run("show", ":src/library.ts"))
                        self.assertIn("继续编辑的问候", (module.path / "src/library.ts").read_text(encoding="utf-8"))
                        self.assertIn("docs/未跟踪 说明.md", module.run("ls-files", "--others", "--exclude-standard", "-z").split("\0"))
                        self.assertEqual(repo.run("config", "-f", ".gitmodules", "--get", "submodule.example-library.url"), "../submodule-remote.git")
                        self.assertEqual(Git(root / "submodule-remote.git", Clock()).run("config", "core.hooksPath"), os.devnull)
                    elif case == Case.DIFF_METADATA:
                        summary = repo.run("-c", "core.quotePath=false", "diff", "--cached", "--summary", "--find-copies")
                        for text in ["100644 => 100755", "100644 => 120000", "delete mode", "rename", "copy", "create mode"]:
                            self.assertIn(text, summary)
                        self.assertEqual(repo.run("rev-parse", "HEAD"), manifest["refs"]["example/metadata-base"])
                        self.assertNotEqual(manifest["refs"]["example/metadata-changes"], manifest["head"])
                        self.assertTrue((repo.path / "元信息/03-文件类型.txt").is_symlink())
                        self.assertEqual(repo.run("rev-list", "--left-right", "--count", "HEAD...@{upstream}"), "0\t0")
                        for name, notices, ends_with_newline in [
                            ("01-补上末尾换行.txt", 1, True),
                            ("02-移除末尾换行.txt", 1, False),
                            ("03-两侧都无换行.txt", 2, False),
                            ("04-新增且无换行.txt", 1, False),
                            ("05-正常换行对照.txt", 0, True),
                        ]:
                            path = "末尾换行/" + name
                            diff = repo.run("diff", "--cached", "--", path)
                            self.assertEqual(diff.count("\\ No newline at end of file"), notices, path)
                            self.assertEqual((repo.path / path).read_bytes().endswith(b"\n"), ends_with_newline, path)
                            self.assertEqual(diff, repo.run("diff", "example/newline-base", "example/newline-changes", "--", path))
                        self.assertEqual(repo.run("rev-parse", "HEAD"), manifest["refs"]["example/newline-base"])
                        self.assertNotEqual(manifest["refs"]["example/newline-changes"], manifest["head"])
                        self.assertEqual(repo.run("rev-list", "--left-right", "--count", "HEAD...@{upstream}"), "0\t0")
                    elif case == Case.STASH_PREVIEW:
                        self.assertIn("MM src/config.ts", repo.run("status", "--porcelain=v1"))
                        self.assertIn("预览演练", repo.run("stash", "list"))
                        self.assertEqual(len(repo.run("rev-list", "--parents", "-n", "1", "stash@{0}").split()), 4)
                        self.assertIn("保存时工作区内容", repo.run("show", "stash@{0}:src/config.ts"))
                        self.assertIn("保存时暂存内容", repo.run("show", "stash@{0}^2:src/config.ts"))
                        self.assertIn("只在原暂存区存在", repo.run("show", "stash@{0}^2:stash/暂存后改回.ts"))
                        self.assertIn("保存时的未跟踪内容", repo.run("show", "stash@{0}^3:stash/中文 新文件.md"))
                        self.assertIn("stash/未跟踪图片.png", repo.run("-c", "core.quotePath=false", "ls-tree", "-r", "--name-only", "stash@{0}^3"))
                    elif case == Case.CHERRY_PICK_BATCH:
                        self.assertEqual(repo.run("status", "--porcelain=v1"), "")
                        self.assertEqual(repo.run("rev-list", "--count", "example/base..feature/batch"), "3")
                        self.assertFalse((repo.path / "src/batch.ts").exists())
                        repo.run("cherry-pick", "example/batch-1", "example/batch-2", "example/batch-3", date=repo.clock.next())
                        self.assertEqual((repo.path / "src/batch.ts").read_text(encoding="utf-8"), 'export const batch = "second";\n')
                        self.assertTrue((repo.path / "docs/batch.md").exists())
                        self.assertEqual(repo.run("status", "--porcelain=v1"), "")
                    else:
                        expected = {Case.CLEAN: "0\t0", Case.AHEAD: "1\t0", Case.BEHIND: "0\t1", Case.DIVERGED: "1\t1"}[case]
                        self.assertEqual(repo.run("rev-list", "--left-right", "--count", "HEAD...@{upstream}"), expected)
                        self.assertEqual(repo.run("status", "--porcelain=v1"), "")
                    self.assertEqual(Git(root / "remote.git", Clock()).run("config", "core.hooksPath"), os.devnull)
                    if case != Case.NO_REMOTE:
                        self.assertEqual(repo.run("remote", "get-url", "origin"), str(root / "remote.git"))
                    profile = (root / "home/config.toml").read_text(encoding="utf-8")
                    self.assertNotIn("check_on_startup", profile)
                    self.assertNotIn("startup_actions", profile)
                    self.assertIn("enabled = false", profile)
                    self.assertIn(json.dumps(str(repo.path), ensure_ascii=False), profile)
                    if case == Case.SUBMODULE:
                        self.assertIn(json.dumps(str(repo.path / MODULE_PATH), ensure_ascii=False), profile)
                        self.assertEqual(manifest["submodules"][0]["path"], MODULE_PATH)

    def test_history_is_reproducible_across_output_directories(self):
        with tempfile.TemporaryDirectory() as temporary:
            for case in [Case.CLEAN, Case.SUBMODULE, Case.DIFF_METADATA, Case.STASH_PREVIEW]:
                with self.subTest(case=case):
                    roots = [generate(Path(temporary) / (case.value + name), case) for name in ["one", "two"]]
                    manifests = [json.loads((root / "manifest.json").read_text(encoding="utf-8")) for root in roots]
                    self.assertEqual(manifests[0], manifests[1])

    def test_submodule_remote_can_initialize_a_clean_clone(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = generate(Path(temporary) / "submodule", Case.SUBMODULE)
            manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
            clone = Git(root / "clone", Clock())
            Git(root, Clock()).run("clone", "--recurse-submodules", str(root / "remote.git"), str(clone.path))
            child = Git(clone.path / MODULE_PATH, Clock())
            self.assertEqual(child.run("rev-parse", "HEAD"), manifest["submodules"][0]["recorded_head"])
            self.assertEqual(clone.run("status", "--porcelain=v1"), "")
            self.assertEqual(child.run("status", "--porcelain=v1"), "")
            self.assertEqual(child.run("show", "HEAD:src/library.ts"), 'export const greeting = "原始问候";')

    def test_existing_directories_and_symlinks_are_never_overwritten(self):
        with tempfile.TemporaryDirectory() as temporary:
            existing = Path(temporary) / "existing"
            existing.mkdir()
            sentinel = existing / "keep.txt"
            sentinel.write_text("keep", encoding="utf-8")
            link = Path(temporary) / "link"
            link.symlink_to(existing, target_is_directory=True)
            for target in [existing, link]:
                with self.subTest(target=target), self.assertRaises(FileExistsError):
                    generate(target, Case.CLEAN)
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep")
            self.assertTrue(link.is_symlink())
            with self.assertRaises(ValueError):
                generate(Path(temporary) / ".git/objects/example", Case.CLEAN)

    def test_callers_git_environment_and_global_configuration_are_not_used(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            trap = Git(root / "trap", Clock())
            trap.path.mkdir()
            trap.run("init", "-b", "main")
            trap.configure()
            trap.write("original.txt", "keep\n")
            trap.commit("original")
            before = trap.run("rev-parse", "HEAD")
            configuration = root / "global.gitconfig"
            configuration.write_text("[user]\n name = WRONG\n email = wrong@example.invalid\n", encoding="utf-8")
            injected = {
                "GIT_DIR": str(trap.path / ".git"), "GIT_WORK_TREE": str(trap.path),
                "GIT_INDEX_FILE": str(root / "injected-index"),
                "GIT_CONFIG_GLOBAL": str(configuration), "GIT_CONFIG_COUNT": "1",
                "GIT_CONFIG_KEY_0": "user.name", "GIT_CONFIG_VALUE_0": "WRONG",
                "GIT_AUTHOR_NAME": "WRONG", "GIT_COMMITTER_NAME": "WRONG",
            }
            for case in [Case.CLEAN, Case.SUBMODULE]:
                with self.subTest(case=case), patch.dict("os.environ", injected):
                    example = generate(root / case.value, case)
                    repos = [example / "repo"]
                    if case == Case.SUBMODULE:
                        repos.append(example / "repo" / MODULE_PATH)
                    for path in repos:
                        repo = Git(path, Clock())
                        self.assertEqual(repo.run("log", "-1", "--format=%an <%ae>"), "GW Example <example@example.invalid>")
                        self.assertEqual(repo.run("config", "protocol.allow"), "never")
                        self.assertEqual(repo.run("config", "protocol.file.allow"), "always")
            self.assertEqual(trap.run("rev-parse", "HEAD"), before)
            self.assertFalse((root / "injected-index").exists())


if __name__ == "__main__":
    unittest.main()
