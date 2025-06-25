set dotenv-load

# Build the Brandwatch query linter
build:
	cargo build --release

# Run all tests
test:
	cargo test

# Lint current directory recursively (default behavior)
lint-dir:
	cargo run --

# Run the linter on a query, file, or directory (auto-detects input type)
lint query-or-file:
	cargo run -- "{{query-or-file}}"

# Validate a query (returns exit code 0 if valid, 1 if invalid)
validate query:
	cargo run -- validate "{{query}}"

# Run the linter in interactive mode
interactive:
	cargo run -- interactive

# Show query examples
examples:
	cargo run -- examples

# Compare our linter with Brandwatch API validation (auto-detects input type)
compare query-or-file:
	@echo "=== Our Linter ==="
	@cargo run -- "{{query-or-file}}" || true
	@echo ""
	@echo "=== Brandwatch API ==="
	@just compare-bw "{{query-or-file}}" || true

# Helper for BW API comparison
compare-bw query-or-file:
	#!/usr/bin/env bash
	if [ -f "{{query-or-file}}" ]; then
		just bw-validate "$(cat '{{query-or-file}}')"
	else
		just bw-validate "{{query-or-file}}"
	fi

# validate a query using the Brandwatch API (for comparison during development)
bw-validate query:
	curl -X POST https://api.brandwatch.com/query-validation \
		-H "authorization: bearer $BW_API_KEY" \
		-H 'Content-Type: application/json' \
		-d '{"booleanQuery": "{{query}}","languages": []}'

# Run comprehensive testing
test-all:
	@echo "Building project..."
	@just build
	@echo "Running unit tests..."
	@just test
	@echo "Testing fixtures..."
	@cargo run -- tests/fixtures
	@echo "All tests completed!"

# Format code
fmt:
	cargo fmt

# Run clippy linter
clippy:
	cargo clippy -- -D warnings

# Clean build artifacts
clean:
	cargo clean

# Install the linter globally
install:
	cargo install --path .

# Development workflow - format, lint, test
dev:
	@just fmt
	@just clippy
	@just test
	@echo "Development checks passed!"
