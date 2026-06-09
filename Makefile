.DEFAULT_GOAL := help

APP := marten
VERSION := 0.1.0

.PHONY: help build run run-release play dev-files clean-dev-files check test fmt lint lint-strict audit ci ci-full release install clean tag

help:
	@printf "\n"
	@printf "Usage: make <target>\n\n"
	@printf "Targets:\n"
	@printf "  %-12s %s\n" "build" "Build the project (debug)."
	@printf "  %-12s %s\n" "run" "Run the project (debug)."
	@printf "  %-12s %s\n" "run-release" "Run the release binary."
	@printf "  %-12s %s\n" "dev-files" "Create untracked files under .marten-dev/ for UI testing (tree, scroll, threshold)."
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

dev-files:
	@mkdir -p .marten-dev/src/utils .marten-dev/tests/fixtures .marten-dev/generated .marten-dev/docs
	@printf "# readme\n\nThis is a placeholder file.\n\nUsed for marten dev testing.\n" \
		> .marten-dev/readme.md
	@seq 1 30  | awk '{print "# line " $$1}' > .marten-dev/config.toml
	@seq 1 150 | awk '{print "// line " $$1}' > .marten-dev/src/main.rs
	@seq 1 40  | awk '{print "// line " $$1}' > .marten-dev/src/lib.rs
	@seq 1 100 | awk '{print "// line " $$1}' > .marten-dev/src/utils/helpers.rs
	@seq 1 25  | awk '{print "// line " $$1}' > .marten-dev/src/utils/validators.rs
	@seq 1 300 | awk '{print "// line " $$1}' > .marten-dev/tests/integration.rs
	@seq 1 80  | awk '{print "line " $$1}'    > .marten-dev/tests/fixtures/sample_output.txt
	@seq 1 16000 | awk '{print "// line " $$1}' > .marten-dev/generated/bindings.rs
	@printf "# long filename\n\nPlaceholder.\n" \
		> ".marten-dev/docs/this-is-a-very-long-filename-to-test-sidebar-truncation-behavior.md"
	@printf "Created dev files under .marten-dev/.\n"

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

tag:
	git tag v$(VERSION)
	git push origin v$(VERSION)
