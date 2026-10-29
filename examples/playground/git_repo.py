"""Isolated system Git commands for disposable examples."""
from __future__ import annotations

from datetime import datetime, timedelta, timezone
from pathlib import Path
import os
import shutil
import subprocess


class Clock:
    def __init__(self):
        self.tick = 0

    def next(self):
        value = datetime(2026, 1, 1, 8, tzinfo=timezone.utc) + timedelta(minutes=self.tick)
        self.tick += 1
        return value.isoformat()


class Git:
    def __init__(self, path: Path, clock: Clock):
        self.path = path
        self.clock = clock
        self.program = shutil.which("git")
        if self.program is None:
            raise RuntimeError("PATH 中未找到 git")

    def run(self, *args: str, expected: int = 0, date: str | None = None) -> str:
        # 清除调用者的仓库重定向、身份与配置注入，仅保留本示例的显式配置。
        environment = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
        environment.update({
            "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_ALLOW_PROTOCOL": "file", "GIT_TERMINAL_PROMPT": "0",
            "GIT_EDITOR": "true", "GIT_SEQUENCE_EDITOR": "true",
        })
        if date is not None:
            environment.update(GIT_AUTHOR_DATE=date, GIT_COMMITTER_DATE=date)
        result = subprocess.run(
            [self.program, "-C", str(self.path), "-c", "core.hooksPath=" + os.devnull,
             "-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false", *args],
            env=environment, stdin=subprocess.DEVNULL, capture_output=True, text=True,
            encoding="utf-8", timeout=30,
        )
        if result.returncode != expected:
            raise RuntimeError(f"git {' '.join(args)} 返回 {result.returncode}，预期 {expected}\n"
                               f"{result.stderr}{result.stdout}")
        return result.stdout.rstrip("\n")

    def configure(self):
        for key, value in {
            "user.name": "GW Example", "user.email": "example@example.invalid",
            "commit.gpgsign": "false", "tag.gpgsign": "false",
            "core.hooksPath": os.devnull, "core.excludesFile": os.devnull,
            "core.autocrlf": "false", "core.precomposeUnicode": "true",
            "protocol.allow": "never", "protocol.file.allow": "always",
        }.items():
            self.run("config", key, value)

    def write(self, name: str, text: str):
        path = self.path / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("w", encoding="utf-8", newline="\n") as output:
            output.write(text)

    def commit(self, message: str):
        self.run("add", "--all")
        self.run("commit", "-m", message, date=self.clock.next())

    def conflict(self, command: str, reference: str):
        self.run(command, "--no-edit", reference, expected=1, date=self.clock.next())
        marker = "CHERRY_PICK_HEAD" if command == "cherry-pick" else "REVERT_HEAD"
        self.run("rev-parse", "--verify", marker)
        if not self.run("ls-files", "--unmerged"):
            raise RuntimeError("场景未生成预期的索引冲突")
