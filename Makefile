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

## ── Performance (Task 266 / 270) ─────────────────────────────────────────────

perf:
	GROK_PERF=1 cargo run -- chat "hello"

perf-interactive:
	GROK_PERF=1 cargo run -- chat --interactive

# Run the criterion micro-benchmarks
bench:
	cargo bench

# Convenience target: capture a quick 5-run baseline for one-shot chat
perf-baseline:
	@echo "=== Quick PERF baseline (5 runs, simple prompt) ==="
	@for i in 1 2 3 4 5; do \
		GROK_PERF=1 cargo run --release -- chat "What is 2+2? Run $$i" 2>&1 | grep '\[perf\]'; \
	done

## ── Binary Size & Build Profiles (Task 272) ──────────────────────────────────

# Standard optimized release
release:
	cargo build --release

# Slim / distribution build (smallest possible binary)
# Uses opt-level=z + fat LTO
release-slim:
	cargo build --profile slim

# Report binary size (useful for tracking Task 272 improvements)
size:
	@echo "=== Release binary size ==="
	@ls -lh target/release/grok-cli* 2>/dev/null || ls -lh target/release/grok-cli.exe 2>/dev/null || echo "Build with 'make release' first"
	@echo ""
	@echo "=== Slim profile size ==="
	@ls -lh target/slim/grok-cli* 2>/dev/null || ls -lh target/slim/grok-cli.exe 2>/dev/null || echo "Build with 'make release-slim' first"

# Full clean + slim release (good before measuring)
release-slim-clean:
	cargo clean
	cargo build --profile slim
	@make size

# Quick compile-time check with dev profile improvements
build-dev:
	cargo build

# Feature-gated build example (tgs-rag is already optional)
build-no-rag:
	cargo build --no-default-features

build-with-rag:
	cargo build --features tgs-rag --release

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
	@echo ""
	@echo "Performance (266/270):"
	@echo "  perf               GROK_PERF single chat"
	@echo "  perf-interactive   GROK_PERF interactive"
	@echo "  bench              cargo bench (criterion)"
	@echo "  perf-baseline      quick 5-run one-shot baseline"
