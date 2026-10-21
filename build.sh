#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$ROOT"
if ! command -v cargo >/dev/null 2>&1; then
    echo '请先安装 Rust，项目工具链版本见 rust-toolchain.toml。' >&2
    exit 2
fi
GW_FLAG_SEPARATOR=$(printf '\037')
GW_PRIVACY_FLAGS="--remap-path-prefix${GW_FLAG_SEPARATOR}${ROOT}=gw${GW_FLAG_SEPARATOR}--remap-path-prefix${GW_FLAG_SEPARATOR}${HOME}=~"
if [ -n "${CARGO_ENCODED_RUSTFLAGS:-}" ]; then
    CARGO_ENCODED_RUSTFLAGS="${CARGO_ENCODED_RUSTFLAGS}${GW_FLAG_SEPARATOR}${GW_PRIVACY_FLAGS}"
else
    CARGO_ENCODED_RUSTFLAGS="$GW_PRIVACY_FLAGS"
fi
export CARGO_ENCODED_RUSTFLAGS
# 同时裁剪内置 libgit2 等 C 依赖的源码路径。
CFLAGS="${CFLAGS:-} -ffile-prefix-map=\"${ROOT}=gw\" -ffile-prefix-map=\"${HOME}=~\""
export CFLAGS
cargo build --release --locked --package gw
mkdir -p dist
cp target/release/gw dist/gw
