#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
GW_DEVELOPER="$ROOT/target/developer-scheduler/debug/app"
if [ ! -x "$GW_DEVELOPER" ]; then
    echo '卸载系统定时任务需要开发构建，请参照 docs/developer-scheduler.md。' >&2
    exit 2
fi
exec "$GW_DEVELOPER" "$@" uninstall
