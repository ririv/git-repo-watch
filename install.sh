#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$ROOT"
if ! command -v go >/dev/null 2>&1; then
    echo '请先安装 Go 1.23 或更高版本。' >&2
    exit 2
fi
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
mkdir -p dist
CGO_ENABLED=0 GOTOOLCHAIN=local GOWORK=off go build -trimpath -buildvcs=false -o dist/gw ./cmd/gw
exec "$ROOT/dist/gw" --home "$ROOT" install
