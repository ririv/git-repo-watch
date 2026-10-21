#!/bin/sh
# 仅在 GitHub 的临时 Linux runner 内安装用户任务；不接触真实仓库。
set -eu
[ "$(uname -s)" = Linux ]
[ "${GITHUB_ACTIONS:-}" = true ]
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
. "$ROOT/application.env"
FIXTURE=$(mktemp -d "${RUNNER_TEMP:?}/repository schedule.XXXXXX")
# runner 为全新用户环境，使用用户管理器启动时的标准目录。
[ -z "${XDG_CONFIG_HOME:-}" ]
[ -z "${XDG_DATA_HOME:-}" ]
UNIT="$HOME/.config/systemd/user/$APP_SERVICE_LABEL"
cleanup() {
    "$ROOT/dist/$APP_COMMAND" --home "$FIXTURE" uninstall || true
    systemctl --user daemon-reload
    rm -rf "$FIXTURE"
}
trap cleanup EXIT HUP INT TERM
cp "$ROOT/config.example.json" "$FIXTURE/config.json"
"$ROOT/dist/$APP_COMMAND" --home "$FIXTURE" install
systemd-analyze --user verify "$UNIT.service" "$UNIT.timer"
systemctl --user is-active "$APP_SERVICE_LABEL.timer"
systemctl --user is-enabled "$APP_SERVICE_LABEL.timer"
"$ROOT/dist/$APP_COMMAND" --home "$FIXTURE" status --json > "$FIXTURE/status.json"
# 安装和重新应用间隔均不得产生检查报告。
[ ! -e "$FIXTURE/var/latest.json" ]
grep -q '21600' "$FIXTURE/status.json"
"$ROOT/dist/$APP_COMMAND" --home "$FIXTURE" install
[ ! -e "$FIXTURE/var/latest.json" ]
# 显式启动一次，仅操作临时本地仓库，自动推送保持关闭。
git init --bare "$FIXTURE/remote.git"
git init "$FIXTURE/repository"
git -C "$FIXTURE/repository" -c user.name=CI -c user.email=ci@example.invalid commit --allow-empty -m initial
git -C "$FIXTURE/repository" remote add origin "$FIXTURE/remote.git"
git -C "$FIXTURE/repository" push -u origin HEAD
"$ROOT/dist/$APP_COMMAND" --home "$FIXTURE" add "$FIXTURE/repository"
systemctl --user start "$APP_SERVICE_LABEL.service"
[ -f "$FIXTURE/var/latest.json" ]
[ "$(systemctl --user show "$APP_SERVICE_LABEL.service" --property=Result --value)" = success ]
[ "$(systemctl --user show "$APP_SERVICE_LABEL.service" --property=ActiveState --value)" = inactive ]
"$ROOT/dist/$APP_COMMAND" --home "$FIXTURE" uninstall
[ ! -e "$UNIT.service" ]
[ ! -e "$UNIT.timer" ]
[ -f "$FIXTURE/config.json" ]
[ -x "$HOME/.local/bin/$APP_COMMAND" ]
trap - EXIT HUP INT TERM
systemctl --user daemon-reload
rm -rf "$FIXTURE"
