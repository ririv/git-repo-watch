"""Build real multi-file conflicts with independent blocks and automatic merge edits."""
from seed import Seed

PATH = "src/合并演练.ts"


def source(side: str) -> str:
    values = {
        "base": ("原始标题", "blue", 2),
        "left": ("当前标题", "green", 3),
        "right": ("传入标题", "purple", 5),
    }
    title, theme, retries = values[side]
    sections = [
        f'export const title = "{title}";\n',
        f'export const theme = "{theme}";\n',
        f"export const retries = {retries};\n",
        f"export const localOnly = {'true' if side == 'left' else 'false'};\n",
        f"export const incomingOnly = {'true' if side == 'right' else 'false'};\n",
    ]
    # 足够长的共同源码让 Git 产生独立冲突块，而非合成一个大块。
    return "".join(
        section + "".join(f"export const shared_{part}_{row} = {row};\n" for row in range(12))
        for part, section in enumerate(sections)
    )


def create(seed: Seed):
    repo = seed.repo
    repo.write(PATH, source("base"))
    repo.commit("准备多块冲突演练")
    repo.run("checkout", "-b", "feature/conflict")
    repo.write(PATH, source("right"))
    repo.write("src/config.ts", 'export const greeting = "修复后的问候";\n')
    repo.commit("演练：传入三个独立修改与问候修复")
    repo.run("tag", "example/conflict")
    repo.run("checkout", "main")
    repo.write(PATH, source("left"))
    repo.write("src/config.ts", 'export const greeting = "当前分支的问候";\n')
    repo.commit("演练：当前分支修改相同位置")
    repo.conflict("cherry-pick", "example/conflict")
