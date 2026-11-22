#!/bin/sh
# 仅在 GitHub 的临时 Linux runner 内安装用户任务；不接触真实仓库。
set -eu
[ "$(uname -s)" = Linux ]
[ "${GITHUB_ACTIONS:-}" = true ]
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
. "$ROOT/application.env"
BINARY=${1:?需要显式传入带 developer-scheduler 特性的构建产物}
FIXTURE=$(mktemp -d "${RUNNER_TEMP:?}/repository schedule.XXXXXX")
# runner 为全新用户环境，使用用户管理器启动时的标准目录。
[ -z "${XDG_CONFIG_HOME:-}" ]
[ -z "${XDG_DATA_HOME:-}" ]
UNIT="$HOME/.config/systemd/user/$APP_SERVICE_LABEL"
PROFILE_MODE=custom
cleanup() {
    case "$PROFILE_MODE" in
        custom) "$BINARY" --home "$FIXTURE" uninstall || true ;;
        standard) "$BINARY" uninstall || true ;;
    esac
    systemctl --user daemon-reload
    rm -rf "$FIXTURE"
}
trap cleanup EXIT HUP INT TERM
assert_no_report() {
    report_status=0
    "$BINARY" --home "$FIXTURE" report --json > "$FIXTURE/report.json" || report_status=$?
    [ "$report_status" -eq 1 ]
}
cp "$ROOT/config.example.toml" "$FIXTURE/config.toml"
"$BINARY" --home "$FIXTURE" install
systemd-analyze --user verify "$UNIT.service" "$UNIT.timer"
grep -q ' scheduled-tick$' "$UNIT.service"
systemctl --user is-active "$APP_SERVICE_LABEL.timer"
systemctl --user is-enabled "$APP_SERVICE_LABEL.timer"
"$BINARY" --home "$FIXTURE" status --json > "$FIXTURE/status.json"
# 安装和重新应用间隔均不得产生检查报告。
assert_no_report
grep -q '21600' "$FIXTURE/status.json"
"$BINARY" --home "$FIXTURE" install
assert_no_report
# 停用保留任务定义；在停用状态应用间隔不得重新启用或触发检查。
"$BINARY" --home "$FIXTURE" schedule disable
[ "$(systemctl --user is-enabled "$APP_SERVICE_LABEL.timer" || true)" = disabled ]
[ "$(systemctl --user is-active "$APP_SERVICE_LABEL.timer" || true)" = inactive ]
[ -f "$UNIT.timer" ]
"$BINARY" --home "$FIXTURE" interval 12h
"$BINARY" --home "$FIXTURE" schedule apply
grep -q '"interval_seconds":43200' "$UNIT.timer"
[ "$(systemctl --user is-enabled "$APP_SERVICE_LABEL.timer" || true)" = disabled ]
[ "$(systemctl --user is-active "$APP_SERVICE_LABEL.timer" || true)" = inactive ]
assert_no_report
"$BINARY" --home "$FIXTURE" schedule enable
systemctl --user is-active "$APP_SERVICE_LABEL.timer"
systemctl --user is-enabled "$APP_SERVICE_LABEL.timer"
"$BINARY" --home "$FIXTURE" interval 6h
"$BINARY" --home "$FIXTURE" schedule apply
assert_no_report
# 显式启动一次，仅操作临时本地仓库，自动推送保持关闭。
git init --bare "$FIXTURE/remote.git"
git init "$FIXTURE/repository"
git -C "$FIXTURE/repository" -c user.name=CI -c user.email=ci@example.invalid commit --allow-empty -m initial
git -C "$FIXTURE/repository" remote add origin "$FIXTURE/remote.git"
git -C "$FIXTURE/repository" push -u origin HEAD
"$BINARY" --home "$FIXTURE" add "$FIXTURE/repository"
case "$PROFILE_MODE" in
    custom)
        "$BINARY" --home "$FIXTURE" interval 60s
        "$BINARY" --home "$FIXTURE" anchor "$(date -d '1 minute ago' '+%Y-%m-%d %H:%M:00')"
        "$BINARY" --home "$FIXTURE" schedule apply ;;
    standard)
        "$BINARY" interval 60s
        "$BINARY" anchor "$(date -d '1 minute ago' '+%Y-%m-%d %H:%M:00')"
        "$BINARY" schedule apply ;;
esac
systemctl --user start "$APP_SERVICE_LABEL.service"
[ -f "$FIXTURE/var/state.sqlite3" ]
"$BINARY" --home "$FIXTURE" report --json > "$FIXTURE/report.json"
[ "$(systemctl --user show "$APP_SERVICE_LABEL.service" --property=Result --value)" = success ]
[ "$(systemctl --user show "$APP_SERVICE_LABEL.service" --property=ActiveState --value)" = inactive ]
"$BINARY" --home "$FIXTURE" uninstall
[ ! -e "$UNIT.service" ]
[ ! -e "$UNIT.timer" ]
[ -f "$FIXTURE/config.toml" ]
[ -x "$HOME/.local/bin/$APP_COMMAND" ]
# 使用含中文和空格的 XDG 目录验证默认配置、后台环境与重复安装。
PROFILE_MODE=standard
export XDG_CONFIG_HOME="$FIXTURE/配置 config"
export XDG_STATE_HOME="$FIXTURE/状态 state"
UNIT="$XDG_CONFIG_HOME/systemd/user/$APP_SERVICE_LABEL"
"$BINARY" init
"$BINARY" add "$FIXTURE/repository"
"$BINARY" install
"$BINARY" install
systemd-analyze --user verify "$UNIT.service" "$UNIT.timer"
grep -q '^Environment=.*XDG_CONFIG_HOME=' "$UNIT.service"
if grep -q ' --home ' "$UNIT.service"; then exit 1; fi
[ ! -e "$XDG_STATE_HOME/$APP_DATA_DIRECTORY/state.sqlite3" ]
case "$PROFILE_MODE" in
    custom)
        "$BINARY" --home "$FIXTURE" interval 60s
        "$BINARY" --home "$FIXTURE" anchor "$(date -d '1 minute ago' '+%Y-%m-%d %H:%M:00')"
        "$BINARY" --home "$FIXTURE" schedule apply ;;
    standard)
        "$BINARY" interval 60s
        "$BINARY" anchor "$(date -d '1 minute ago' '+%Y-%m-%d %H:%M:00')"
        "$BINARY" schedule apply ;;
esac
systemctl --user start "$APP_SERVICE_LABEL.service"
[ -f "$XDG_STATE_HOME/$APP_DATA_DIRECTORY/state.sqlite3" ]
"$BINARY" report --json > "$FIXTURE/standard-report.json"
"$BINARY" schedule disable
"$BINARY" interval 12h
"$BINARY" schedule apply
[ "$(systemctl --user is-enabled "$APP_SERVICE_LABEL.timer" || true)" = disabled ]
"$BINARY" schedule enable
"$BINARY" uninstall
[ ! -e "$UNIT.service" ]
[ ! -e "$UNIT.timer" ]
[ -f "$XDG_CONFIG_HOME/$APP_DATA_DIRECTORY/config.toml" ]
trap - EXIT HUP INT TERM
systemctl --user daemon-reload
rm -rf "$FIXTURE"
