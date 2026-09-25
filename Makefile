.PHONY: build run test fmt clippy check release clean help

# Default target
all: build

help:
	@echo "Available commands:"
	@echo "  make build    - Build the project in debug mode"
	@echo "  make run      - Run the project"
	@echo "  make test     - Run the tests of every workspace crate (gitwig and gitwig-core)"
	@echo "  make fmt      - Format the code using cargo fmt"
	@echo "  make clippy   - Run clippy for linting"
	@echo "  make check    - Check the code for compilation errors without building"
	@echo "  make release  - Build the project in release mode"
	@echo "  make clean    - Clean the project"

build:
	cargo build

run:
	cargo run

# --workspace: a plain `cargo test` at the root only tests the `gitwig` package,
# so gitwig-core's tests never ran (in CI either, which uses this target).
test:
	cargo test --workspace

fmt:
	cargo fmt

clippy:
	cargo clippy

check:
	cargo check

release:
	cargo build --release

publish:
	cargo publish -p gitwig-core && cargo publish

clean:
	cargo clean

lint:
	cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used

fmt-check:
	cargo fmt -- --check

ci: fmt-check lint test
