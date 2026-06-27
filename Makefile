.PHONY: build run test fmt clippy check release clean help

# Default target
all: build

help:
	@echo "Available commands:"
	@echo "  make build    - Build the project in debug mode"
	@echo "  make run      - Run the project"
	@echo "  make test     - Run tests"
	@echo "  make fmt      - Format the code using cargo fmt"
	@echo "  make clippy   - Run clippy for linting"
	@echo "  make check    - Check the code for compilation errors without building"
	@echo "  make release  - Build the project in release mode"
	@echo "  make clean    - Clean the project"

build:
	cargo build

run:
	cargo run

test:
	cargo test

fmt:
	cargo fmt

clippy:
	cargo clippy

check:
	cargo check

release:
	cargo build --release

publish:
	cargo publish

clean:
	cargo clean

lint:
	cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used

fmt-check:
	cargo fmt -- --check

ci: fmt-check lint test
