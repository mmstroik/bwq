use crate::error::{Position, Span};
use lsp_types::{Position as LspPosition, Range as LspRange};

pub fn position_to_lsp(pos: &Position) -> LspPosition {
    LspPosition {
        line: pos.line.saturating_sub(1) as u32,
        character: pos.column.saturating_sub(1) as u32,
    }
}

pub fn span_to_range(span: &Span) -> LspRange {
    LspRange {
        start: position_to_lsp(&span.start),
        end: position_to_lsp(&span.end),
    }
}

pub fn range_from_coords(
    start_line: u32,
    start_char: u32,
    end_line: u32,
    end_char: u32,
) -> LspRange {
    LspRange {
        start: LspPosition {
            line: start_line,
            character: start_char,
        },
        end: LspPosition {
            line: end_line,
            character: end_char,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_conversion() {
        let pos = Position::new(1, 1, 0);
        let lsp_pos = position_to_lsp(&pos);
        assert_eq!(lsp_pos.line, 0);
        assert_eq!(lsp_pos.character, 0)
    }

    #[test]
    fn test_span_conversion() {
        let span = Span::new(Position::new(1, 1, 0), Position::new(1, 5, 4));
        let range = span_to_range(&span);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 4);
    }
}
