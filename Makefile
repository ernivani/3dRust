.PHONY: all build run clean check

all: build

build:
	cargo build

run:
	cargo run

clean:
	cargo clean

check:
	cargo check

release:
	cargo build --release
	cargo run --release

test:
	cargo test

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check