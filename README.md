# bw-bool

brandwatch boolean search query linter in rust.

## install

```bash
cargo install --path .
```

## usage

```bash
# lint a single query
bw-bool lint "apple AND juice"

# validate query (exit code 0/1)
bw-bool validate "apple AND juice"

# lint queries from file
bw-bool file test_queries.txt

# interactive mode
bw-bool interactive

# show examples
bw-bool examples
```

## justfile commands

```bash
# build and test
just build
just test

# lint queries
just lint "your query here"
just lint-file

# compare with brandwatch api
just compare "your query here"

# development
just dev
```

## features

- comprehensive lexer for all brandwatch operators
- ast-based parser with validation
- detailed error reporting with position info
- performance warnings
- cli with multiple output formats
- integration tests with real queries

## operators supported

- boolean: `AND`, `OR`, `NOT`
- proximity: `~`, `NEAR/x`, `NEAR/xf`
- wildcards: `*`, `?`
- case sensitive: `{word}`
- fields: `title:`, `site:`, `author:`, etc.
- ranges: `[x TO y]`
- comments: `<<<text>>>`
- special chars: `#hashtag`, `@mention`

see `bw-bool examples` for comprehensive list.