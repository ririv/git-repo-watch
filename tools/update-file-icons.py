#!/usr/bin/env python3
"""Generate deterministic icon tables from a pinned revision without executing Lua."""
import argparse
import json
from pathlib import Path
import re
import urllib.request

PATTERN = re.compile(
    r'^\s*\["([^"]+)"\]\s*=\s*\{\s*icon\s*=\s*"([^"]+)",\s*color\s*=\s*"(#[0-9A-Fa-f]{6})"',
    re.MULTILINE,
)
ASCII_LOWER = str.maketrans('ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz')


def download(revision, name):
    url = f'https://raw.githubusercontent.com/nvim-tree/nvim-web-devicons/{revision}/{name}'
    request = urllib.request.Request(url, headers={'User-Agent': 'gw-file-icons-updater'})
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read().decode('utf-8')


def table(source, name):
    entries = PATTERN.findall(source)
    if not entries or any(line.lstrip().startswith('[') and not PATTERN.match(line)
                          for line in source.splitlines()):
        raise ValueError(f'{name}: 存在未识别的图标条目')
    values = {}
    for key, glyph, color in entries:
        if len(glyph) != 1:
            raise ValueError(f'{name}: {key} 的图标不是单个字符')
        key = key.translate(ASCII_LOWER)
        value = (glyph, color.lower())
        if key in values and values[key] != value:
            raise ValueError(f'{name}: {key} 的大小写规则冲突')
        values[key] = value
    lines = ['#[rustfmt::skip]', f'pub(super) const {name}: &[(&str, Glyph)] = &[']
    for key, (glyph, color) in sorted(values.items()):
        rgb = ', '.join(str(int(color[offset:offset + 2], 16)) for offset in (1, 3, 5))
        lines.append(f'    ({json.dumps(key, ensure_ascii=False)}, Glyph::new(\'\\u{{{ord(glyph):x}}}\', {rgb})),')
    lines.append('];')
    return '\n'.join(lines), len(values)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--revision', required=True, help='上游完整 Git 提交哈希')
    args = parser.parse_args()
    if not re.fullmatch('[0-9a-f]{40}', args.revision):
        parser.error('--revision 必须是完整的小写 Git 提交哈希')
    tables = []
    counts = {}
    for kind, name in [('FILENAMES', 'icons_by_filename'), ('EXTENSIONS', 'icons_by_file_extension')]:
        source = download(args.revision, f'lua/nvim-web-devicons/default/{name}.lua')
        content, count = table(source, kind)
        tables.append(content)
        counts[kind] = count
    license_text = download(args.revision, 'LICENSE')
    root = Path(__file__).resolve().parents[1]
    target = root / 'crates/tui/src/file_icons'
    target.mkdir(exist_ok=True)
    header = f'//! Generated from nvim-web-devicons {args.revision}; update with tools/update-file-icons.py.\nuse super::Glyph;\n\n'
    (target / 'data.rs').write_text(header + '\n\n'.join(tables) + '\n')
    (target / 'LICENSE').write_text(license_text)
    print(json.dumps({'revision': args.revision, **counts}))


if __name__ == '__main__':
    main()
