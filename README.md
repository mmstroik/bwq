# bwq

brandwatch boolean search query linter and LSP written in rust.

## install

```bash
cargo install bwq
```

## usage

```bash
# lint directory (recursively) or specific file(s)
bwq check                        # lint all `.bwq` files in the current directory (and subdirectories)
bwq check path/to/queries/       # lint all `.bwq` files in `path/to/queries/` (and subdirectories)
bwq check path/to/query.bwq      # lint `query.bwq`
bwq check path/to/queries/*.txt  # lint all `.txt` files in `path/to/queries/`

# lint query strings directly
bwq check --query "term1 AND wildcardterm*"

# lint both .txt and .bwq files in current directory
bwq check -e txt -e bwq 

# show all options
bwq check --help
```

## bw operator support

- boolean: `AND`, `OR`, `NOT`
- proximity: `~`, `NEAR/x`, `NEAR/xf`
- wildcards: `*`, `?`
- fields: `title:`, `site:`, `rating:[1 TO 5]`
- special: `{case}`, `#hashtag`, `@mention`, `<<<comments>>>`

run `bwq examples` for more
