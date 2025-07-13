set dotenv-load

build:
	cargo build

clean:
	cargo clean

format:
	cargo fmt --all

lint:
	cargo clippy -- -D warnings

lint-fix:
	cargo clippy --fix --allow-dirty --allow-staged

dev:
	@just lint
	@just format
	@just test
	@echo "Development checks passed!"

test:
	cargo nextest run --workspace --status-level slow

bwq-check *files:
	cargo run --bin bwq -- check {{files}}

bwq-check-q query:
	cargo run --bin bwq -- check --query '{{query}}'

bwq-check-json *files:
	cargo run --bin bwq -- check --output-format json {{files}}

bwq-check-q-json query:
	cargo run --bin bwq -- check --output-format json --query '{{query}}'

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