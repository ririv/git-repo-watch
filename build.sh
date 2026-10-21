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
# 发布构建始终编入 Oniguruma，避免受本机系统库和动态链接环境影响。
RUSTONIG_SYSTEM_LIBONIG=0
RUSTONIG_DYNAMIC_LIBONIG=0
export RUSTONIG_SYSTEM_LIBONIG RUSTONIG_DYNAMIC_LIBONIG
cargo build --release --locked --package app
mkdir -p dist
cp target/release/app "dist/$APP_COMMAND"
cp THIRD_PARTY_NOTICES.md dist/THIRD_PARTY_NOTICES.md
