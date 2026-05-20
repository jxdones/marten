.DEFAULT_GOAL := help

APP := marten

.PHONY: help build run run-release play dev-files clean-dev-files check test fmt lint lint-strict audit ci ci-full release install clean

help:
	@printf "\n"
	@printf "Usage: make <target>\n\n"
	@printf "Targets:\n"
	@printf "  %-12s %s\n" "build" "Build the project (debug)."
	@printf "  %-12s %s\n" "run" "Run the project (debug)."
	@printf "  %-12s %s\n" "run-release" "Run the release binary."
	@printf "  %-12s %s\n" "dev-files" "Create dummy untracked files for local files panel testing."
	@printf "  %-12s %s\n" "clean-dev-files" "Remove dummy files created by dev-files."
	@printf "  %-12s %s\n" "check" "Type-check and compile without linking."
	@printf "  %-12s %s\n" "test" "Run tests."
	@printf "  %-12s %s\n" "fmt" "Format Rust code."
	@printf "  %-12s %s\n" "lint" "Run clippy and fail on warnings."
	@printf "  %-12s %s\n" "lint-strict" "Run clippy with additional pedantic lints before committing."
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

run-release:
	@if [ ! -x target/release/marten ]; then \
		echo "Binary not found. Building release first..."; \
		$(MAKE) release; \
	fi
	@./target/release/marten

dev-files:
	@mkdir -p .marten-dev/nested
	@printf "Dummy unstaged content for marten development.\n" > .marten-dev/unstaged.txt
	@printf "Nested dummy file for panel truncation checks.\n" > .marten-dev/nested/very-long-file-name-for-files-panel.txt
	@printf "Created dummy untracked files under .marten-dev/.\n"

clean-dev-files:
	rm -rf .marten-dev

check:
	cargo check --all-targets

test:
	cargo test

fmt:
	cargo fmt --all

lint:
	cargo clippy --all-targets -- -D warnings

lint-strict:
	cargo clippy --all-targets --all-features -- \
		-D warnings \
		-W clippy::pedantic \
		-W clippy::nursery \
		-A clippy::missing_errors_doc

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
