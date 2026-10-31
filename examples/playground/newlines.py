"""Compose end-of-file newline samples into an existing playground scenario."""
from git_repo import Git


CHANGES = (
    ("01-补上末尾换行.txt", "相同文字", "相同文字\n"),
    ("02-移除末尾换行.txt", "相同文字\n", "相同文字"),
    ("03-两侧都无换行.txt", "旧版内容", "新版内容"),
    ("05-正常换行对照.txt", "旧版内容\n", "新版内容\n"),
)


def write_before(repo: Git):
    for name, before, _ in CHANGES:
        repo.write("末尾换行/" + name, before)


def write_after(repo: Git):
    for name, _, after in CHANGES:
        repo.write("末尾换行/" + name, after)
    repo.write("末尾换行/04-新增且无换行.txt", "新增内容")
