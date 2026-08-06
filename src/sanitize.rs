//! Sanitize untrusted text before rendering in the terminal.

use strip_ansi_escapes::strip;
use unicode_width::UnicodeWidthChar;

/// Remove ANSI escape sequences and replace dangerous control characters.
pub fn sanitize_text(input: &str) -> String {
    let stripped = strip(input.as_bytes());
    let text = String::from_utf8_lossy(&stripped);
    sanitize_controls(&text)
}

/// Like [`sanitize_text`], but also collapses runs of whitespace for table cells.
pub fn sanitize_cell(input: &str) -> String {
    let base = sanitize_text(input);
    let mut out = String::with_capacity(base.len());
    let mut prev_ws = false;
    for ch in base.chars() {
        if ch.is_whitespace() {
            if !prev_ws {
                out.push(' ');
                prev_ws = true;
            }
        } else {
            out.push(ch);
            prev_ws = false;
        }
    }
    out.trim().to_string()
}

fn sanitize_controls(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\n' | '\t' => out.push(' '),
            '\r' => {}
            c if c.is_control() => out.push('\u{FFFD}'),
            c => {
                // Replace zero-width / non-printing that can confuse layout.
                if UnicodeWidthChar::width(c).unwrap_or(0) == 0 && c != ' ' {
                    out.push('\u{FFFD}');
                } else {
                    out.push(c);
                }
            }
        }
    }
    out
}

/// Truncate to roughly `max_width` display columns.
pub fn truncate_width(input: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let mut width = 0usize;
    let mut out = String::new();
    for ch in input.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + w > max_width {
            break;
        }
        out.push(ch);
        width += w;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ansi() {
        let raw = "\x1b[31mred\x1b[0m text";
        assert_eq!(sanitize_text(raw), "red text");
    }

    #[test]
    fn replaces_controls() {
        let raw = "hello\x07world\x1b";
        let s = sanitize_text(raw);
        assert!(!s.contains('\x07'));
        assert!(s.contains('h'));
    }

    #[test]
    fn sanitizes_cell_whitespace() {
        assert_eq!(sanitize_cell("a \n\t  b"), "a b");
    }

    #[test]
    fn truncate_respects_budget() {
        assert_eq!(truncate_width("abcdef", 3), "abc");
    }

    #[test]
    fn truncate_does_not_panic_on_multibyte() {
        // Previously `&s[..120]` panicked when byte 120 split a UTF-8 char.
        let s = "á".repeat(100);
        let t = truncate_width(&s, 50);
        assert!(!t.is_empty());
        assert!(t.chars().count() <= 50);
    }
}
