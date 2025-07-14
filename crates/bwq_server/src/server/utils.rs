use lsp_types::Position;

/// Convert LSP position to byte position in text
pub fn lsp_position_to_byte_position(text: &str, position: Position) -> usize {
    let line_offset = position.line as usize;
    let utf16_char_offset = position.character as usize;

    // Find line boundaries by examining actual line endings in the text
    let mut line_start_positions = vec![0];
    let mut pos = 0;
    let text_bytes = text.as_bytes();

    while pos < text_bytes.len() {
        if text_bytes[pos] == b'\n' {
            line_start_positions.push(pos + 1);
            pos += 1;
        } else if text_bytes[pos] == b'\r'
            && pos + 1 < text_bytes.len()
            && text_bytes[pos + 1] == b'\n'
        {
            line_start_positions.push(pos + 2);
            pos += 2;
        } else {
            pos += 1;
        }
    }

    if line_offset >= line_start_positions.len() {
        return text.len();
    }

    let line_start = line_start_positions[line_offset];
    let line_end = if line_offset + 1 < line_start_positions.len() {
        // Get the start of the next line and work backwards to find line content end
        let next_line_start = line_start_positions[line_offset + 1];
        if next_line_start >= 2
            && text_bytes.get(next_line_start - 2) == Some(&b'\r')
            && text_bytes.get(next_line_start - 1) == Some(&b'\n')
        {
            next_line_start - 2 // CRLF
        } else if next_line_start >= 1 && text_bytes.get(next_line_start - 1) == Some(&b'\n') {
            next_line_start - 1 // LF
        } else {
            next_line_start
        }
    } else {
        text.len()
    };

    let line = &text[line_start..line_end];

    // Convert UTF-16 character offset to UTF-8 byte offset within the line
    let line_byte_offset = {
        let mut utf16_count = 0;
        let mut byte_offset = 0;

        for ch in line.chars() {
            if utf16_count >= utf16_char_offset {
                break;
            }
            byte_offset += ch.len_utf8();
            utf16_count += ch.len_utf16();
        }

        // Clamp to line length if offset is beyond line end
        byte_offset.min(line.len())
    };

    line_start + line_byte_offset
}
