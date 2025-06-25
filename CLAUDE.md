# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based linter and parser for Brandwatch query files (.bwq). It validates query syntax, provides detailed error reporting with position information, and supports all Brandwatch operators including boolean operations, proximity searches, field operations, and wildcards.

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
```

**File Processing:** For complex multi-line queries or shell escaping issues, use .bwq files:
```bash
bwq-lint query.bwq                  # Auto-detects and lints .bwq file
bwq-lint tests/fixtures             # Auto-detects and processes directory  
bwq-lint "*.bwq"                    # Auto-detects glob pattern
```

Installation and usage:
```bash
just install              # Install globally via cargo
bwq-lint "query"          # Use installed binary (auto-detects input type)
bwq-lint interactive      # Interactive mode
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

**Field Validation:** Comprehensive field-specific validation (ratings 0-5, coordinates, language codes, etc.) with both errors and performance warnings.

**Position Tracking:** All errors include precise position information (line, column, offset) via the `Span` system for excellent user experience.

## Testing Strategy

- **Unit tests** in each module for core functionality
- **Integration tests** (`tests/integration_tests.rs`) with real-world queries and edge cases
- **Test fixtures** (`tests/fixtures/*.bwq`) for multi-line queries and complex scenarios
- **File processing tests** validate .bwq file handling and directory processing

## Key Files

- `brandwatch-query-operators.md` - Complete operator reference (do not modify)
- `justfile` - Development commands and build tools
- `tests/fixtures/*.bwq` - Test fixture files for complex queries

## Validation Rules

The validator implements strict Brandwatch-specific rules:
- Boolean operators must be capitalized (AND, OR, NOT)
- Wildcards cannot start words (*invalid)
- Rating values must be 0-5
- Coordinate ranges for latitude/longitude
- Proximity distance limits and performance warnings
- Mixed operators require parentheses (AND/OR, NEAR/OR combinations)
- Field-specific value validation (language: ISO codes, etc.)

## Notes for Future Development

When modifying the parser, remember that NOT is binary, not unary. When adding new operators, follow the existing pattern in `ast.rs` for type definitions and `lexer.rs` for tokenization. The validator should include both error conditions (malformed syntax) and warnings (performance concerns, style suggestions).

## Claude Guidance

- Always reference `./brandwatch-query-operators.md` for operator syntax and behavior
- Use test fixtures in `tests/fixtures/` for testing complex scenarios
- When adding validation rules, ensure alignment with actual Brandwatch API behavior