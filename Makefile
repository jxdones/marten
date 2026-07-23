.DEFAULT_GOAL := help

APP := marten
VERSION := 0.1.0

.PHONY: help build run run-release play demo dev-files clean-dev-files check test fmt lint lint-strict audit ci ci-full release install clean tag

help:
	@printf "\n"
	@printf "Usage: make <target>\n\n"
	@printf "Targets:\n"
	@printf "  %-12s %s\n" "build" "Build the project (debug)."
	@printf "  %-12s %s\n" "run" "Run the project (debug)."
	@printf "  %-12s %s\n" "run-release" "Run the release binary."
	@printf "  %-12s %s\n" "demo" "Generate the README demo GIF with VHS."
	@printf "  %-12s %s\n" "dev-files" "Create a synthetic git repo at /tmp/libstr for UI testing."
	@printf "  %-12s %s\n" "clean-dev-files" "Remove the test repo at /tmp/libstr."
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
	@printf "  %-12s %s\n" "tag" "Create and push git tag v\$(VERSION)."
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

demo: dev-files
	cd $(DEV_REPO) && env -u NO_COLOR COLORTERM=truecolor vhs $(CURDIR)/assets/demo.tape --output $(CURDIR)/assets/marten.gif

DEV_REPO ?= /tmp/libstr

dev-files:
	@bash scripts/test-repo.sh $(DEV_REPO)

clean-dev-files:
	rm -rf $(DEV_REPO)

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
	cargo install --path . --locked

clean:
	cargo clean

tag:
	git tag v$(VERSION)
	git push origin v$(VERSION)
