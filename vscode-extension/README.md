# bwq

brandwatch boolean search query linter and language server for vs code.

## install

requires the `bwq` binary:

```bash
cargo install bwq
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

**errors (E001-E017):**

- E001: lexer errors (invalid characters, malformed tokens)
- E002: parser errors (syntax issues, parsing failures)  
- E003: validation errors (general validation issues)
- E004: invalid boolean case
- E005: unbalanced parentheses
- E006: invalid wildcard placement
- E007: invalid proximity operator syntax
- E008: invalid field operator syntax
- E009: invalid range syntax
- E010: unexpected token
- E011: expected token but found something else
- E012: field validation errors
- E013: proximity operator errors
- E014: range validation errors
- E015: operator mixing errors
- E016: pure negative query errors
- E017: invalid field operator spacing

**warnings (W001-W003):**

- W001: potential typo (suggestions, implicit AND usage)
- W002: deprecated operator
- W003: performance warning (short wildcards)

## config

- `bwq.serverPath`: path to bwq executable (default: "bwq")
- `bwq.trace.server`: trace communication with language server
