"""Build real file-header changes in both the index and tagged history."""
import os

from seed import Seed
from newlines import write_before, write_after


def prepare(seed: Seed):
    repo = seed.repo
    repo.run("config", "core.filemode", "false" if os.name == "nt" else "true")
    repo.run("config", "core.symlinks", "true")
    repo.run("config", "diff.renames", "copies")
    repo.write("元信息/01-普通修改.ts", "export const message = 'before';\n")
    repo.write("元信息/02-可执行权限.sh", "#!/bin/sh\nprintf 'metadata demo\\n'\n")
    (repo.path / "元信息/02-可执行权限.sh").chmod(0o644)
    repo.write("元信息/03-文件类型.txt", "普通文件将变为符号链接。\n")
    repo.write("元信息/04-待删除.md", "# 删除演练\n")
    content = "".join(f"第 {number:02} 行：用于对比重命名和复制的内容相似度。\n" for number in range(30))
    repo.write("元信息/05-重命名前.md", "# 重命名演练\n" + content)
    repo.write("元信息/06-复制来源.md", "# 复制演练\n" + content)
    (repo.path / "元信息/08-二进制.dat").write_bytes(b"\x00before\x00metadata\n")
    write_before(repo)
    repo.commit("元信息演练：建立比较基准")
    repo.run("tag", "example/metadata-base")
    repo.run("tag", "example/newline-base")
    repo.run("push", "origin", "main")

    repo.write("元信息/01-普通修改.ts", "export const message = 'after';\n")
    if os.name != "nt":
        (repo.path / "元信息/02-可执行权限.sh").chmod(0o755)
    (repo.path / "元信息/03-文件类型.txt").unlink()
    (repo.path / "元信息/03-文件类型.txt").symlink_to("01-普通修改.ts")
    (repo.path / "元信息/04-待删除.md").unlink()
    repo.run("mv", "--", "元信息/05-重命名前.md", "元信息/05-重命名后.md")
    repo.write("元信息/05-重命名后.md", "# 重命名后的标题\n" + content)
    repo.write("元信息/06-复制目标.md", "# 复制演练\n" + content)
    repo.write("元信息/06-复制来源.md", "# 复制后继续编辑来源\n" + content)
    repo.write("元信息/07-新增文件.md", "# 新增演练\n\n查看新增文件的类型和权限。\n")
    (repo.path / "元信息/08-二进制.dat").write_bytes(b"\x00after\x00metadata\n")
    write_after(repo)
    repo.run("add", "--all")
    repo.run("update-index", "--chmod=+x", "--", "元信息/02-可执行权限.sh")
    repo.run("commit", "-m", "元信息演练：权限、类型、重命名、复制及二进制变更", date=repo.clock.next())
    repo.run("tag", "example/metadata-changes")
    repo.run("tag", "example/newline-changes")
    repo.run("push", "origin", "--tags")
    # 保留同一份已暂存改动，历史提交由标签保持可达；不覆盖其它演练目录。
    repo.run("reset", "--soft", "example/metadata-base")
