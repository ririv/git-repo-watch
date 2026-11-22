#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$ROOT/application.env"
case "$(uname -s)" in
    Darwin) tools="/usr/bin/git /usr/bin/ssh /usr/bin/osascript" ;;
    Linux) tools="/usr/bin/git /usr/bin/ssh" ;;
    *) echo "暂不支持此系统" >&2; exit 2 ;;
esac
for tool in $tools; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "缺少系统命令：$tool" >&2
        exit 2
    fi
done
"$ROOT/build.sh"
"$ROOT/dist/$APP_COMMAND" "$@" migrate config
"$ROOT/dist/$APP_COMMAND" "$@" config >/dev/null
exec "$ROOT/dist/$APP_COMMAND" "$@" install
