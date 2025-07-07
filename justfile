set dotenv-load

build:
	cargo build --release

clean:
	cargo clean

format:
	cargo fmt

lint:
	cargo clippy -- -D warnings

lint-fix:
	cargo clippy --fix --allow-dirty --allow-staged

dev:
	@just lint
	@just format
	@just test-all
	@echo "Development checks passed!"

test:
	cargo test

test-all:
	@echo "Building project..."
	@just build
	@echo "Running tests..."
	@just test
	@echo "Testing fixtures..."
	@cargo run -- check --exit-zero tests/fixtures
	@echo "All tests completed!"

# Compare our linter with Brandwatch API validation
compare query-or-file:
	@echo "=== Our Linter ==="
	@just compare-our '{{query-or-file}}' || true
	@echo ""
	@echo "=== Brandwatch API ==="
	@just compare-bw '{{query-or-file}}' || true

compare-our query-or-file:
	#!/usr/bin/env bash
	if [ -f "{{query-or-file}}" ]; then
		cargo run -- check "{{query-or-file}}"
	else
		cargo run -- check --query '{{query-or-file}}'
	fi

compare-bw query-or-file:
	#!/usr/bin/env bash
	if [ -f "{{query-or-file}}" ]; then
		just bw-validate '$(cat '{{query-or-file}}')'
	else
		just bw-validate '{{query-or-file}}'
	fi

bw-validate query:
	curl -X POST https://api.brandwatch.com/query-validation \
		-H "authorization: bearer $BW_API_KEY" \
		-H 'Content-Type: application/json' \
		-d '{"booleanQuery": "{{query}}","languages": []}'