use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Remove terminal protocols before layout. Never pass untrusted escapes through.
pub fn sanitize(text: &str) -> String {
    enum State {
        Text,
        Escape,
        EscapeIntermediate,
        Csi,
        String(bool),
        StringEscape(bool),
    }
    let mut state = State::Text;
    let mut output = String::with_capacity(text.len());
    for ch in text.chars() {
        state = match state {
            State::Text => match ch {
                '\x1b' => State::Escape,
                '\u{9b}' => State::Csi,
                '\u{9d}' => State::String(true),
                '\u{90}' | '\u{98}' | '\u{9e}' | '\u{9f}' => State::String(false),
                // The two Unicode line separators are not control characters,
                // but some renderers break a line on them, so they become a
                // space like the ASCII line breaks. Other line-ish characters
                // that *are* control characters (NEL, VT, FF) are dropped by
                // the control check below; both outcomes keep them from
                // splitting a row, but this is not one uniform rule.
                '\n' | '\r' | '\t' | '\u{2028}' | '\u{2029}' => {
                    output.push(' ');
                    State::Text
                }
                // Control characters, plus every Bidi_Control character: the
                // implicit marks (ALM, LRM, RLM), the embedding and override
                // range, and the isolate range. They occupy no cells, so they
                // would reorder a hostname or username without changing width.
                ch if ch.is_control()
                    || matches!(
                        ch,
                        '\u{061c}'
                            | '\u{200e}'
                            | '\u{200f}'
                            | '\u{202a}'..='\u{202e}'
                            | '\u{2066}'..='\u{2069}'
                    ) =>
                {
                    State::Text
                }
                ch => {
                    output.push(ch);
                    State::Text
                }
            },
            State::Escape => match ch {
                '[' => State::Csi,
                ']' => State::String(true),
                'P' | 'X' | '^' | '_' => State::String(false),
                ' '..='/' => State::EscapeIntermediate,
                '\x1b' => State::Escape,
                _ => State::Text,
            },
            State::EscapeIntermediate => {
                if ('0'..='~').contains(&ch) {
                    State::Text
                } else {
                    State::EscapeIntermediate
                }
            }
            State::Csi => {
                if ('@'..='~').contains(&ch) {
                    State::Text
                } else if ch == '\x1b' {
                    State::Escape
                } else {
                    State::Csi
                }
            }
            State::String(osc) => match ch {
                '\u{9c}' => State::Text,
                '\x07' if osc => State::Text,
                '\x1b' => State::StringEscape(osc),
                _ => State::String(osc),
            },
            State::StringEscape(osc) => {
                if ch == '\\' || ch == '\u{9c}' || (osc && ch == '\x07') {
                    State::Text
                } else {
                    State::String(osc)
                }
            }
        };
    }
    output
}

pub fn width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

/// Wrap without breaking grapheme clusters. Width is always at least one cell.
pub fn wrap(text: &str, columns: usize) -> Vec<String> {
    let columns = columns.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let cells = width(grapheme);
        if cells > columns {
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            lines.push("?".to_owned());
            used = 0;
            continue;
        }
        if used + cells > columns && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            used = 0;
        }
        line.push_str(grapheme);
        used += cells;
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_display_cells_and_preserves_graphemes() {
        assert_eq!(wrap("a中e\u{301}👩‍🔬z", 3), ["a中", "e\u{301}👩‍🔬", "z"]);
        assert!(wrap("中中文", 1).iter().all(|line| width(line) <= 1));
        assert_eq!(wrap("e\u{301}x", 1), ["e\u{301}", "x"]);
    }

    #[test]
    fn untrusted_sequences_cannot_control_terminal() {
        assert_eq!(
            sanitize("a\x1b[31mred\x1b[0m\x1b]52;c;secret\x07b"),
            "aredb"
        );
        assert_eq!(sanitize("x\u{9b}2J\u{202e}y\n\tz\x00"), "xy  z");
        // Unicode line separators would split a row in some renderers.
        assert_eq!(sanitize("a\u{2028}b\u{2029}c"), "a b c");
        // Every Bidi_Control character is removed, not just the overrides.
        for mark in ['\u{061c}', '\u{200e}', '\u{200f}', '\u{2066}', '\u{2069}'] {
            assert_eq!(sanitize(&format!("a{mark}b")), "ab", "{mark:?}");
        }
        assert_eq!(sanitize("ok\x1bPmalicious\x1b\\end"), "okend");
    }
}
