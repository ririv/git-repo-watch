#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$ROOT"
. "$ROOT/application.env"
if ! command -v cargo >/dev/null 2>&1; then
    echo '请先安装 Rust，项目工具链版本见 rust-toolchain.toml。' >&2
    exit 2
fi
BUILD_FLAG_SEPARATOR=$(printf '\037')
BUILD_PRIVACY_FLAGS="--remap-path-prefix${BUILD_FLAG_SEPARATOR}${ROOT}=source${BUILD_FLAG_SEPARATOR}--remap-path-prefix${BUILD_FLAG_SEPARATOR}${HOME}=~"
if [ -n "${CARGO_ENCODED_RUSTFLAGS:-}" ]; then
    CARGO_ENCODED_RUSTFLAGS="${CARGO_ENCODED_RUSTFLAGS}${BUILD_FLAG_SEPARATOR}${BUILD_PRIVACY_FLAGS}"
else
    CARGO_ENCODED_RUSTFLAGS="$BUILD_PRIVACY_FLAGS"
fi
export CARGO_ENCODED_RUSTFLAGS
# 同时裁剪内置 libgit2 等 C 依赖的源码路径。
CFLAGS="${CFLAGS:-} -ffile-prefix-map=\"${ROOT}=source\" -ffile-prefix-map=\"${HOME}=~\""
export CFLAGS
cargo build --release --locked --package app
mkdir -p dist
cp target/release/app "dist/$APP_COMMAND"
