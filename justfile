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
	cargo test -q --workspace


bwq-check *files:
	cargo run --bin bwq -- check {{files}}

bwq-check-q query:
	cargo run --bin bwq -- check --query '{{query}}'

bwq-check-json *files:
	cargo run --bin bwq -- check --output-format json {{files}}

bwq-check-q-json query:
	cargo run --bin bwq -- check --output-format json --query '{{query}}'


# Compare our linter with Brandwatch API validation
compare:
	@echo "=== Our Linter ==="
	@just compare-our "$1" || true
	@echo ""
	@echo "=== Brandwatch API ==="
	@just compare-bw "$1" || true

compare-our:
	#!/usr/bin/env bash
	if [ -f "$1" ]; then
		just bwq check "$1"
	else
		just bwq check --query "$1"
	fi

compare-bw:
	#!/usr/bin/env bash
	if [ -f "$1" ]; then
		just bw-validate "$(cat "$1")"
	else
		just bw-validate "$1"
	fi

bw-validate:
	curl -X POST https://api.brandwatch.com/query-validation \
		-H "authorization: bearer $BW_API_KEY" \
		-H 'Content-Type: application/json' \
		-d '{"booleanQuery": "'"$1"'","languages": []}'
