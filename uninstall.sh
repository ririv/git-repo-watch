#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$ROOT/application.env"
exec "$ROOT/dist/$APP_COMMAND" --home "$ROOT" uninstall
