"""Create one stash with independently inspectable saved layers and different current files."""
from seed import Seed, png


def create(seed: Seed):
    repo = seed.repo
    repo.write("stash/暂存后改回.ts", 'export const value = "基准内容";\n')
    repo.commit("准备 stash 分层预览基准")
    repo.run("push", "origin", "main")
    repo.write("src/config.ts", 'export const greeting = "保存时暂存内容";\n')
    repo.write("stash/暂存后改回.ts", 'export const value = "只在原暂存区存在";\n')
    repo.run("add", "--", "src/config.ts", "stash/暂存后改回.ts")
    repo.write("src/config.ts", 'export const greeting = "保存时工作区内容";\n')
    repo.write("stash/暂存后改回.ts", 'export const value = "基准内容";\n')
    repo.run("mv", "--", "docs/旧名称.md", "docs/新名称.md")
    (repo.path / "docs/待删除.md").unlink()
    repo.write("stash/中文 新文件.md", "# 保存时的未跟踪内容\n\n" + "未跟踪说明。\n" * 30)
    (repo.path / "stash/未跟踪图片.png").write_bytes(png())
    (repo.path / "stash/二进制.dat").write_bytes(b"\0saved binary\xff\n")
    repo.run("stash", "push", "--include-untracked", "-m", "预览演练：暂存、未暂存和未跟踪内容", date=repo.clock.next())
    # 故意保留不同的当前索引与工作区，验证预览只读取 stash 对象。
    repo.write("src/config.ts", 'export const greeting = "当前索引内容，不应出现在预览";\n')
    repo.run("add", "--", "src/config.ts")
    repo.write("src/config.ts", 'export const greeting = "当前工作区内容，不应出现在预览";\n')
