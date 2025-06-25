# bwq-lint

brandwatch boolean search query linter and LSP written in rust.

## install

```bash
cargo install bwq-lint
```

## usage

```bash
# lints current directory (recursively) by default
bwq-lint

# auto-detects input type (string, file, dir, glob)
bwq-lint "apple AND juice"     # query string
bwq-lint query.bwq             # file
bwq-lint tests/fixtures        # directory (recursive)
bwq-lint "*.bwq"               # glob pattern

# warnings shown by default, use --no-warnings to suppress
bwq-lint --no-warnings
```

## operators

- boolean: `AND`, `OR`, `NOT`
- proximity: `~`, `NEAR/x`, `NEAR/xf`
- wildcards: `*`, `?`
- fields: `title:`, `site:`, `rating:[1 TO 5]`
- special: `{case}`, `#hashtag`, `@mention`, `<<<comments>>>`

run `bwq-lint examples` for more
