set dotenv-load

# Build the Brandwatch boolean query linter
build:
	cargo build --release

# Run all tests
test:
	cargo test

# Run the linter on a specific query
lint query:
	cargo run -- lint "{{query}}" --warnings

# Validate a query (returns exit code 0 if valid, 1 if invalid)
validate query:
	cargo run -- validate "{{query}}"

# Run the linter on the test queries file
lint-file:
	cargo run -- file test_queries.txt --warnings

# Run the linter in interactive mode
interactive:
	cargo run -- interactive --warnings

# Show query examples
examples:
	cargo run -- examples

# Compare our linter with Brandwatch API validation
compare query:
	@echo "=== Our Linter ==="
	@cargo run -- lint "{{query}}" --warnings || true
	@echo ""
	@echo "=== Brandwatch API ==="
	@just bw-validate "{{query}}" || true

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
	@echo "Testing example queries..."
	@just lint-file
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
