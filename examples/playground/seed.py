"""Build a reproducible history shared by the manual scenarios."""
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import shutil
import struct
import zlib

from git_repo import Clock, Git


ASSETS = (
    "README.md", "src/main.ts", "src/config.ts", "docs/使用指南.md",
    "docs/带 空格的说明.md", "媒体/status.svg",
)


def png() -> bytes:
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    # 固定 RGBA 色条，用于验证尺寸、透明度和像素预览，无外部图片依赖。
    colors = [(99, 168, 248, 255), (130, 210, 160, 255), (240, 150, 180, 128)]
    rows = b"".join(b"\0" + b"".join(bytes(colors[x // 64]) for x in range(192)) for _ in range(96))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 192, 96, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


@dataclass
class Seed:
    repo: Git
    peer: Git
    remote: Path
    submodules: tuple[Seed, ...] = ()


def create(root: Path) -> Seed:
    clock = Clock()
    repo = Git(root / "repo", clock)
    repo.path.mkdir()
    repo.run("init", "-b", "main")
    repo.configure()
    for name in ASSETS:
        asset = Path(__file__).parent / "assets" / name
        destination = repo.path / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(asset, destination)
    (repo.path / "媒体/colors.png").write_bytes(png())
    repo.write(".gitignore", "cache/\n*.tmp\n")
    repo.write("docs/待删除.md", "# 即将删除的演示文件\n")
    repo.write("docs/旧名称.md", "# 重命名演示\n")
    repo.commit("初始化示例应用与预览素材")
    repo.run("tag", "example/base")
    repo.run("checkout", "-b", "feature/ui")
    repo.write("src/ui.ts", 'export const theme = "blue";\n')
    repo.commit("新增界面主题")
    repo.write("src/ui.ts", 'export const theme: "blue" | "green" = "blue";\n')
    repo.commit("为主题添加类型约束")
    repo.run("checkout", "-b", "feature/fix", "example/base")
    repo.write("src/config.ts", 'export const greeting = "修复后的问候";\n')
    repo.commit("修复问候文案")
    repo.run("tag", "example/cherry-pick")
    repo.run("checkout", "main")
    repo.write("src/main.ts", (repo.path / "src/main.ts").read_text(encoding="utf-8")
               + '\nexport const version = "1.1.0";\n')
    repo.commit("增加应用版本")
    repo.run("merge", "--no-ff", "-m", "合并界面主题", "feature/ui", date=clock.next())
    repo.run("tag", "example/merge")
    repo.write("src/revert-me.ts", 'export const experimental = true;\n')
    repo.commit("启用可撤销的实验功能")
    repo.run("tag", "example/revert")
    remote = root / "remote.git"
    remote.mkdir()
    remote_repo = Git(remote, clock)
    remote_repo.run("init", "--bare", "-b", "main")
    remote_repo.configure()
    repo.run("remote", "add", "origin", str(remote))
    repo.run("push", "--all", "origin")
    repo.run("push", "--tags", "origin")
    repo.run("branch", "--set-upstream-to=origin/main", "main")
    peer_path = root / "peer"
    Git(root, clock).run("clone", str(remote), str(peer_path))
    peer = Git(peer_path, clock)
    peer.configure()
    return Seed(repo, peer, remote)
