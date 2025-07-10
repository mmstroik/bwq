set dotenv-load

# Convenient alias for running the CLI during development
bwq *args:
	cargo run --bin bwq -- {{args}}

build:
	cargo build

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
	@just test
	@echo "Development checks passed!"

test:
	cargo test -q --workspace

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
		just bwq check "{{query-or-file}}"
	else
		just bwq check --query '{{query-or-file}}'
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

# Release commands (only CLI crate is published)
release level="patch":
	cd crates/bwq && cargo release {{level}}

release-dry level="patch":
	cd crates/bwq && cargo release {{level}} --dry-run