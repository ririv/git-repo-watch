#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$ROOT"
. "$ROOT/application.env"
if [ ! -f config.json ]; then
    echo '请先执行 cp config.example.json config.json，并显式配置仓库。' >&2
    exit 2
fi
for tool in /usr/bin/git /usr/bin/ssh /usr/bin/osascript /bin/launchctl /usr/bin/plutil; do
    if [ ! -x "$tool" ]; then
        echo "缺少系统命令：$tool" >&2
        exit 2
    fi
done
"$ROOT/build.sh"
exec "$ROOT/dist/$APP_COMMAND" --home "$ROOT" install
