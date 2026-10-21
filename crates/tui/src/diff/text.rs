//! Preserve tab stops while stripping terminal commands across token boundaries.
use ratatui::{style::Style, text::Span};
use unicode_width::UnicodeWidthChar;
use vte::{Parser, Perform};

pub(super) fn spans<'a>(parts: impl IntoIterator<Item = (&'a str, Style)>) -> Vec<Span<'static>> {
    let mut parser = Parser::new();
    let mut painter = Painter {
        spans: Vec::new(),
        column: 0,
        style: Style::default(),
    };
    for (text, style) in parts {
        painter.style = style;
        parser.advance(&mut painter, text.as_bytes());
    }
    painter.spans
}

struct Painter {
    spans: Vec<Span<'static>>,
    column: usize,
    style: Style,
}
impl Painter {
    fn append(&mut self, text: &str) {
        if let Some(last) = self.spans.last_mut()
            && last.style == self.style
        {
            last.content.to_mut().push_str(text);
        } else {
            self.spans.push(Span::styled(text.to_owned(), self.style));
        }
    }
}
impl Perform for Painter {
    fn print(&mut self, ch: char) {
        let ch = if ch.is_control() { ' ' } else { ch };
        self.append(ch.encode_utf8(&mut [0; 4]));
        self.column += UnicodeWidthChar::width(ch).unwrap_or(0);
    }
    fn execute(&mut self, byte: u8) {
        if byte == b'\t' {
            let spaces = 4 - self.column % 4;
            self.append(&" ".repeat(spaces));
            self.column += spaces;
        }
    }
}
