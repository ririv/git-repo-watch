.PHONY: build test check install
build:
	./build.sh

test:
	cargo test --workspace --locked

check:
	cargo fmt --all --check
	cargo clippy --workspace --locked --all-targets -- -D warnings

install:
	./install.sh
