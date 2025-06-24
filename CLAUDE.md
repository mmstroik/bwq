# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based linter and parser for Brandwatch boolean search queries. It validates query syntax, provides detailed error reporting with position information, and supports all Brandwatch operators including boolean operations, proximity searches, field operations, and wildcards.

## Key Commands

Development workflow:
```bash
just dev          # Format, clippy, and test in one command
just build        # Build release version
just test         # Run all tests
cargo test --test integration_tests test_name  # Run specific integration test
```

Linting and validation:
```bash
just lint "query"         # Lint a query with warnings
just validate "query"     # Validate query (exit code 0/1)
just compare "query"      # Compare linter vs Brandwatch API
```

**Shell Escaping Issues:** If encountering shell parsing issues with special characters (like `~`, quotes, etc.), create a `.bq` file with the query content and test using:
```bash
bw-bool lint "$(cat query.bq)"     # Test from file
bw-bool lint *.bq                  # Test multiple .bq files
```

Installation and usage:
```bash
just install              # Install globally via cargo
bw-bool lint "query"      # Use installed binary
bw-bool interactive       # Interactive mode
```

## Architecture

The codebase follows a classic compiler architecture:

**Core Pipeline:** `src/lib.rs` orchestrates the main flow:
1. **Lexer** (`src/lexer.rs`) - Tokenizes query strings into structured tokens
2. **Parser** (`src/parser.rs`) - Builds AST from tokens using recursive descent parsing
3. **Validator** (`src/validator.rs`) - Validates AST with 60+ comprehensive rules

**Key Components:**
- **AST Definitions** (`src/ast.rs`) - Complete type definitions for all Brandwatch operators
- **Error System** (`src/error.rs`) - Position-accurate error reporting with spans
- **CLI Interface** (`src/main.rs`) - Multiple output formats (human, JSON)

**Critical Implementation Details:**

**Binary NOT Operator:** Unlike typical boolean parsers, Brandwatch treats NOT as a binary operator (like AND/OR), not unary. The parser implements this correctly in `parse_not_expression()`.

**Implicit AND Support:** The parser supports space-separated terms as implicit AND operations (for Brandwatch compatibility) but generates warnings encouraging explicit operators. This is handled in `parse_and_expression()` via `is_implicit_and_candidate()`.

**Field Validation:** Comprehensive field-specific validation (ratings 1-5, coordinates, language codes, etc.) with both errors and performance warnings.

**Position Tracking:** All errors include precise position information (line, column, offset) via the `Span` system for excellent user experience.

## Testing Strategy

- **Unit tests** in each module for core functionality
- **Integration tests** (`tests/integration_tests.rs`) with real-world queries including user-provided edge cases
- **API comparison** (`test_alignment.sh`) to validate against actual Brandwatch API behavior
- **Implicit AND behavior tests** specifically validate the space-separated term handling

## Key Files

- `brandwatch-query-operators.md` - Complete operator reference (do not modify)
- `justfile` - Development commands and API comparison tools
- `test_queries.txt` - Sample queries for testing (if exists)

## Validation Rules

The validator implements strict Brandwatch-specific rules:
- Boolean operators must be capitalized (AND, OR, NOT)
- Wildcards cannot start words (*invalid)
- Rating values must be 1-5
- Coordinate ranges for latitude/longitude
- Proximity distance limits and performance warnings
- Field-specific value validation (gender: F/M, language: ISO codes, etc.)

## Notes for Future Development

When modifying the parser, remember that NOT is binary, not unary. When adding new operators, follow the existing pattern in `ast.rs` for type definitions and `lexer.rs` for tokenization. The validator should include both error conditions (malformed syntax) and warnings (performance concerns, style suggestions).

## Claude Guidance

- It is very very very important that you use `just compare "query"` often rather than making assumptions

## Memories

- Reference/read `./brandwatch-query-operators.md` often for guidance on list of operators and general syntax