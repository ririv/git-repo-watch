//! Semantic links remain separate from terminal cells and native OSC output.
mod detect;
mod native;
mod opener;
mod text;
use engine::desktop::Target;
pub(crate) use native::Native;
pub(crate) use opener::Opener;
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Text as PlainText},
    widgets::{Paragraph, Widget, Wrap},
};
use std::{collections::BTreeMap, sync::Arc};
pub(crate) use text::{Anchor, Line, Text};
#[derive(Debug, Default, Clone)]
pub(crate) struct Map {
    pub cells: BTreeMap<(u16, u16), Arc<Target>>,
    enabled: bool,
    pub paths: Vec<std::path::PathBuf>,
}
impl Map {
    pub fn reset(&mut self, enabled: bool) {
        self.cells.clear();
        self.enabled = enabled;
    }
    pub fn text(&self, text: ratatui::text::Text<'static>) -> Text {
        Text::new(text, &self.paths)
    }
    pub fn hit(&self, point: Position) -> Option<Arc<Target>> {
        self.cells.get(&(point.y, point.x)).cloned()
    }
    fn publish(&mut self, frame: &mut Frame, shadow: Buffer, targets: &[Arc<Target>]) {
        if !self.enabled {
            return;
        }
        for y in shadow.area.y..shadow.area.bottom() {
            for x in shadow.area.x..shadow.area.right() {
                let cell = &shadow[(x, y)];
                let Color::Rgb(r, g, b) = cell.fg else {
                    continue;
                };
                let id = ((usize::from(r) << 16) | (usize::from(g) << 8) | usize::from(b))
                    .checked_sub(1);
                if let Some(target) = id.and_then(|id| targets.get(id))
                    && !cell.symbol().trim().is_empty()
                {
                    self.cells.insert((y, x), target.clone());
                    frame.buffer_mut()[(x, y)]
                        .modifier
                        .insert(Modifier::UNDERLINED);
                }
            }
        }
    }
    pub fn paragraph(&mut self, frame: &mut Frame, area: Rect, text: &Text, scroll: usize) {
        let scroll = scroll.min(u16::MAX as usize) as u16;
        frame.render_widget(
            Paragraph::new(text.plain())
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0)),
            area,
        );
        if !self.enabled || area.is_empty() {
            return;
        }
        let mut targets = Vec::new();
        let marked = PlainText::from(
            text.lines
                .iter()
                .map(|line| mark(line, &mut targets))
                .collect::<Vec<_>>(),
        );
        if targets.is_empty() {
            return;
        }
        let mut shadow = Buffer::empty(area);
        Paragraph::new(marked)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0))
            .render(area, &mut shadow);
        self.publish(frame, shadow, &targets);
    }
    pub fn line(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        line: &Line,
        fixed: usize,
        offset: usize,
    ) {
        frame.render_widget(
            crate::horizontal::pinned(&line.text, fixed, offset, area.width as usize),
            area,
        );
        if !self.enabled || area.is_empty() {
            return;
        }
        let mut targets = Vec::new();
        let marked = mark(line, &mut targets);
        let mut shadow = Buffer::empty(area);
        crate::horizontal::pinned(&marked, fixed, offset, area.width as usize)
            .render(area, &mut shadow);
        self.publish(frame, shadow, &targets);
    }
}
// 只在独立布局缓冲区中用颜色编码局部索引，不修改真实文字或把索引输出到终端。
fn mark(line: &Line, targets: &mut Vec<Arc<Target>>) -> ratatui::text::Line<'static> {
    let indexes = line
        .anchors
        .iter()
        .map(|anchor| {
            targets.push(anchor.target.clone());
            targets.len()
        })
        .collect::<Vec<_>>();
    let mut column = 0;
    let spans = line
        .text
        .styled_graphemes(Style::default())
        .map(|grapheme| {
            use unicode_width::UnicodeWidthStr;
            let start = column;
            column += grapheme.symbol.width();
            let id = line
                .anchors
                .iter()
                .zip(&indexes)
                .find(|(anchor, _)| anchor.columns.start <= start && column <= anchor.columns.end)
                .map(|(_, id)| *id);
            let style = id.map_or(Style::default(), |id| {
                Style::default().fg(Color::Rgb((id >> 16) as u8, (id >> 8) as u8, id as u8))
            });
            Span::styled(grapheme.symbol.to_owned(), style)
        })
        .collect::<Vec<_>>();
    ratatui::text::Line::from(spans)
}

impl crate::Screen {
    pub(crate) fn poll_links(&mut self) -> bool {
        let Some(result) = self.link_opener.poll() else {
            return false;
        };
        self.message = match result {
            Ok(()) => i18n::tr!(LinkRequested).into(),
            Err(error) => format!("{}: {error}", i18n::tr!(LinkOpenFailed)),
        };
        true
    }
}
