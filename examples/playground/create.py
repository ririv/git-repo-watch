#!/usr/bin/env python3
"""Create a persistent, disposable Git playground without touching user profiles."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import shlex
import sys

from scenarios import APPLY, Case
import seed


PROJECT = Path(__file__).resolve().parents[2]


def generate(directory: Path, case: Case) -> Path:
    directory = directory.expanduser().absolute()
    if ".git" in directory.parts:
        raise ValueError("不能在 Git 元数据目录中创建示例")
    directory.parent.mkdir(parents=True, exist_ok=True)
    # mkdir 是占有目标目录的唯一入口；已有目录、文件或符号链接均拒绝覆盖。
    directory.mkdir()
    directory = directory.resolve()
    example = seed.create(directory)
    APPLY[case](example)
    home = directory / "home"
    home.mkdir()
    repositories = [example.repo, *(module.repo for module in example.submodules)]
    paths = json.dumps([str(repo.path) for repo in repositories], ensure_ascii=False)
    (home / "config.toml").write_text(
        '# 演练专用配置，不安装定时任务，不调用 AI。\n'
        'interval_seconds = 21600\n'
        f'repositories = {paths}\n'
        'excluded_roots = []\n'
        'startup_actions = ["inspect"]\n'
        'scheduled_actions = ["inspect"]\n'
        'max_concurrency = 1\n'
        'check_on_startup = false\n'
        'text_wrap = false\n'
        '\n[ai]\nenabled = false\n', encoding="utf-8",
    )
    tags = ["example/base", "example/cherry-pick", "example/revert", "example/merge"]
    if case == Case.CHERRY_PICK_BATCH:
        tags.extend(f"example/batch-{index}" for index in range(1, 4))
    if case == Case.SUBMODULE:
        tags.append("example/submodule")
    if case == Case.DIFF_METADATA:
        tags.extend(["example/metadata-base", "example/metadata-changes"])
    manifest = {
        "format": 1, "case": case.value,
        "repo": "repo", "remote": "remote.git", "peer": "peer", "home": "home",
        "head": example.repo.run("rev-parse", "HEAD"),
        "refs": {tag: example.repo.run("rev-parse", tag) for tag in tags},
        "status": example.repo.run("status", "--porcelain=v1"),
        "submodules": [
            {
                "path": module.repo.path.relative_to(example.repo.path).as_posix(),
                "remote": module.remote.relative_to(directory).as_posix(),
                "peer": module.peer.path.relative_to(directory).as_posix(),
                "head": module.repo.run("rev-parse", "HEAD"),
                "recorded_head": example.repo.run(
                    "rev-parse", "HEAD:" + module.repo.path.relative_to(example.repo.path).as_posix(),
                ),
                "status": module.repo.run("status", "--porcelain=v1"),
            }
            for module in example.submodules
        ],
    }
    (directory / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8",
    )
    return directory


def main():
    parser = argparse.ArgumentParser(description="生成可丢弃的 GW Git 演练仓库")
    parser.add_argument("--case", type=Case, choices=list(Case), default=Case.WORKSPACE,
                        help="场景，默认 workspace")
    parser.add_argument("--directory", type=Path, help="新目录；默认 work/playground/<场景>")
    args = parser.parse_args()
    directory = args.directory if args.directory is not None else PROJECT / "work/playground" / args.case.value
    try:
        directory = generate(directory, args.case)
    except FileExistsError as error:
        parser.exit(1, f"创建失败：{error}\n已有内容不会覆盖；请选择另一个 --directory。\n")
    except (OSError, RuntimeError, ValueError) as error:
        parser.exit(1, f"创建失败：{error}\n")
    repo = shlex.quote(str(directory / "repo"))
    home = shlex.quote(str(directory / "home"))
    print(f"场景：{args.case.value}\n目录：{directory}\n")
    print(f"cd {repo}\ngw --home {home}")
    print(f"\n从独立仓库列表启动：gw --home {home} repos")
    if args.case == Case.SUBMODULE:
        print("\n演练配置已登记父仓库和子模块；可分别查看引用变化与内部文件改动。")


if __name__ == "__main__":
    if sys.version_info < (3, 9):
        raise SystemExit("需要 Python 3.9 或更新版本")
    main()
