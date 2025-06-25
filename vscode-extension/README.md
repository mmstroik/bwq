# bwq-lint

brandwatch boolean search query linter for vs code.

## install

requires the `bwq-lint` binary:

```bash
cargo install bwq-lint
```

## features

syntax highlighting for all brandwatch operators

real-time error detection and performance warnings  

precise error positioning with detailed messages

## operators

boolean: `AND`, `OR`, `NOT`
proximity: `~`, `NEAR/x`, `NEAR/xf`
wildcards: `*`, `?`
fields: `title:`, `site:`, `rating:[1 TO 5]`
special: `{case}`, `#hashtag`, `@mention`, `<<<comments>>>`

## error codes

**errors (E001-E011):**

- E001: lexer errors (invalid characters)
- E002: parser errors (syntax issues)  
- E003: validation errors
- E004: invalid boolean case (use AND/OR/NOT)
- E005: unbalanced parentheses
- E006: invalid wildcard placement
- E007: invalid proximity operator syntax
- E008: invalid field operator syntax
- E009: invalid range syntax
- E010: unexpected token
- E011: expected token

**warnings (W001-W003):**

- W001: potential typo
- W002: deprecated operator
- W003: performance warning

## config

- `bwqLint.serverPath`: path to bwq-lint executable (default: "bwq-lint")
- `bwqLint.trace.server`: trace communication with language server
