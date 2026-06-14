# grok-cli Makefile
# Requires GNU Make. On Windows: install via Chocolatey (`choco install make`)
# or Scoop (`scoop install make`), or run the cargo commands directly.

.PHONY: build release test test-integration test-all test-coverage lint clean help

## ── Build ────────────────────────────────────────────────────────────────────

build:
	cargo build

release:
	cargo build --release

## ── Tests ────────────────────────────────────────────────────────────────────

# Unit tests only (lib + bin internal #[cfg(test)] blocks)
test:
	cargo test --lib

# Full integration harness (task 148) — offline, no API key needed
test-integration:
	cargo test --test task_tools_tests \
	           --test file_tools_tests \
	           --test subsystem_tests  \
	           --test cli_smoke_tests

# All tests: unit + integration (excludes #[ignore] network tests)
test-all:
	cargo test

# Run every test including network/API ones (requires GROK_API_KEY)
test-network:
	cargo test -- --include-ignored

# Coverage report via cargo-tarpaulin (install: cargo install cargo-tarpaulin)
test-coverage:
	cargo tarpaulin \
	  --out Html \
	  --output-dir target/coverage \
	  --exclude-files "src/bin/*" "src/main.rs" \
	  --timeout 120
	@echo "Report: target/coverage/tarpaulin-report.html"

# Coverage via cargo-llvm-cov (install: cargo install cargo-llvm-cov)
test-coverage-llvm:
	cargo llvm-cov --html --output-dir target/llvm-cov
	@echo "Report: target/llvm-cov/index.html"

## ── Lint ─────────────────────────────────────────────────────────────────────

lint:
	cargo clippy --all-targets -- -D warnings

lint-fix:
	cargo clippy --all-targets --fix

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

## ── Clean ────────────────────────────────────────────────────────────────────

clean:
	cargo clean

## ── Help ─────────────────────────────────────────────────────────────────────

help:
	@echo "grok-cli make targets:"
	@echo "  build              cargo build (debug)"
	@echo "  release            cargo build --release"
	@echo "  test               unit tests only"
	@echo "  test-integration   offline integration harness (task 148)"
	@echo "  test-all           all tests (unit + integration)"
	@echo "  test-network       all including #[ignore] network tests"
	@echo "  test-coverage      HTML coverage via tarpaulin"
	@echo "  test-coverage-llvm HTML coverage via llvm-cov"
	@echo "  lint               cargo clippy -D warnings"
	@echo "  lint-fix           clippy --fix"
	@echo "  fmt                cargo fmt"
	@echo "  fmt-check          cargo fmt --check"
	@echo "  clean              cargo clean"
