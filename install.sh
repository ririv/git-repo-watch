#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$ROOT"
. "$ROOT/application.env"
if [ ! -f config.json ]; then
    echo '请先执行 cp config.example.json config.json，并显式配置仓库。' >&2
    exit 2
fi
case "$(uname -s)" in
    Darwin) tools="/usr/bin/git /usr/bin/ssh /usr/bin/osascript /bin/launchctl /usr/bin/plutil" ;;
    Linux) tools="/usr/bin/git /usr/bin/ssh systemctl" ;;
    *) echo "暂不支持此系统" >&2; exit 2 ;;
esac
for tool in $tools; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "缺少系统命令：$tool" >&2
        exit 2
    fi
done
"$ROOT/build.sh"
"$ROOT/dist/$APP_COMMAND" --home "$ROOT" migrate
exec "$ROOT/dist/$APP_COMMAND" --home "$ROOT" install
