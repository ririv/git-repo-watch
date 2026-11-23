#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$ROOT/application.env"
DEVELOPER_COMMAND="$ROOT/target/developer-scheduler/debug/$APP_COMMAND"
if [ ! -x "$DEVELOPER_COMMAND" ]; then
    echo '卸载系统定时任务需要开发构建，请参照 docs/developer-scheduler.md。' >&2
    exit 2
fi
exec "$DEVELOPER_COMMAND" "$@" uninstall
