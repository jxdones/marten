.DEFAULT_GOAL := help

APP := marten

.PHONY: help build run play check test fmt lint audit ci ci-full release install clean

help:
	@printf "\n"
	@printf "Usage: make <target>\n\n"
	@printf "Targets:\n"
	@printf "  %-12s %s\n" "build" "Build the project (debug)."
	@printf "  %-12s %s\n" "check" "Type-check and compile without linking."
	@printf "  %-12s %s\n" "test" "Run tests."
	@printf "  %-12s %s\n" "fmt" "Format Rust code."
	@printf "  %-12s %s\n" "lint" "Run clippy and fail on warnings."
	@printf "  %-12s %s\n" "audit" "Run cargo audit."
	@printf "  %-12s %s\n" "ci" "Run fmt + lint + test checks."
	@printf "  %-12s %s\n" "ci-full" "Run fmt + lint + test + audit checks."
	@printf "  %-12s %s\n" "release" "Build optimized release binary."
	@printf "  %-12s %s\n" "install" "Install binary from this path."
	@printf "  %-12s %s\n" "clean" "Remove Cargo build artifacts."
	@printf "\n"

build:
	cargo build

run:
	cargo run --

check:
	cargo check --all-targets

test:
	cargo test

fmt:
	cargo fmt --all

lint:
	cargo clippy --all-targets -- -D warnings

audit:
	cargo audit

ci: fmt lint test

ci-full: fmt lint test audit

release:
	cargo build --release

install:
	cargo install --path .

clean:
	cargo clean

