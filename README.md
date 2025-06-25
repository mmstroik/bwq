# bwq-lint

brandwatch query linter written in rust.

## install

```bash
cargo install --path .
```

## usage

```bash
# auto-detects input type (file, dir, string)
bwq-lint "apple AND juice"     # query string
bwq-lint query.bwq             # file
bwq-lint tests/fixtures        # directory
bwq-lint "*.bwq"               # glob pattern

# explicit commands still work
bwq-lint validate "apple AND juice"
bwq-lint interactive
bwq-lint examples
```

## operators

boolean: `AND`, `OR`, `NOT`
proximity: `~`, `NEAR/x`, `NEAR/xf`
wildcards: `*`, `?`
fields: `title:`, `site:`, `rating:[1 TO 5]`
special: `{case}`, `#hashtag`, `@mention`, `<<<comments>>>`

run `bwq-lint examples` for more
