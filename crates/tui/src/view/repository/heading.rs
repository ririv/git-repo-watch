//! Responsive repository panel titles and list options.
use super::*;
use crate::{pointer, repository_list::Menu};

const SEPARATOR: &str = " · ";

struct Options {
    filter: Line<'static>,
    sort: Line<'static>,
}
impl Options {
    fn line(&self) -> Line<'static> {
        Line::from(
            self.filter
                .spans
                .iter()
                .cloned()
                .chain(std::iter::once(Span::styled(
                    SEPARATOR,
                    Style::default().fg(MUTED),
                )))
                .chain(self.sort.spans.iter().cloned())
                .collect::<Vec<_>>(),
        )
    }
    fn buttons(&self, map: &mut pointer::Map, x: u16, y: u16) {
        let filter_width = self.filter.width() as u16;
        map.button(
            Rect::new(x, y, filter_width, 1),
            pointer::Action::RepositoryMenu(Menu::Filter),
        );
        map.button(
            Rect::new(
                x + filter_width + Line::from(SEPARATOR).width() as u16,
                y,
                self.sort.width() as u16,
                1,
            ),
            pointer::Action::RepositoryMenu(Menu::Sort),
        );
    }
}

enum Placement {
    Title,
    Content,
    Hidden,
}

fn options(filter: Filter, order: Order) -> Options {
    let field = |label, value: &'static str| {
        Line::from(vec![
            Span::styled(
                i18n::tf!(FieldLabel, label = label),
                Style::default().fg(MUTED),
            ),
            Span::styled(value, Style::default().fg(ACCENT)),
        ])
    };
    Options {
        filter: field(i18n::tr!(RepositoryFilterLabel), filter.label()),
        sort: field(i18n::tr!(RepositorySortLabel), order.label()),
    }
}

pub(super) fn render(
    frame: &mut Frame,
    title: String,
    filter: Filter,
    order: Order,
    area: Rect,
    pointer: &mut pointer::Map,
) -> Rect {
    // 极矮窗口直接显示列表，避免边框和标题耗尽全部内容行。
    if area.height < 3 {
        return area;
    }
    let controls = options(filter, order);
    let mut options = controls.line();
    // 按面板实际列宽计算，标题之间留一格边框，选项两侧各留一格空白。
    let needed = Line::from(title.as_str()).width() + options.width() + 3;
    let border = block(title).padding(ratatui::widgets::Padding::left(1));
    let content = border.inner(area);
    let placement = if needed <= usize::from(area.width.saturating_sub(2)) {
        Placement::Title
    } else if content.height >= 3 && options.width() <= usize::from(content.width) {
        Placement::Content
    } else {
        Placement::Hidden
    };
    match placement {
        Placement::Title => {
            controls.buttons(pointer, area.right() - options.width() as u16 - 2, area.y);
            options.spans.insert(0, Span::raw(" "));
            options.spans.push(Span::raw(" "));
            frame.render_widget(border.title(options.right_aligned()), area);
            content
        }
        Placement::Content => {
            controls.buttons(pointer, content.x, content.y);
            frame.render_widget(border, area);
            let parts =
                Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(content);
            frame.render_widget(Paragraph::new(options), parts[0]);
            parts[1]
        }
        Placement::Hidden => {
            frame.render_widget(border, area);
            content
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn bilingual_options_move_inside_at_the_title_width_boundary_without_overlap() {
        for language in [i18n::Language::English, i18n::Language::Chinese] {
            let _language = i18n::using(language);
            for (filter, order) in [
                (Filter::All, Order::Priority),
                (Filter::WithoutUpstream, Order::Updated),
            ] {
                let title = format!("{} · 26", i18n::tr!(Repositories));
                let options = options(filter, order).line();
                let needed = Line::from(title.as_str()).width() + options.width() + 5;
                let mut terminal = Terminal::new(TestBackend::new(120, 8)).unwrap();
                for (width, row) in [(needed as u16, 0), (needed as u16 - 1, 1)] {
                    let mut table = Rect::ZERO;
                    terminal
                        .draw(|frame| {
                            table = render(
                                frame,
                                title.clone(),
                                filter,
                                order,
                                Rect::new(0, 0, width, 8),
                                &mut crate::pointer::Map::default(),
                            );
                        })
                        .unwrap();
                    let buffer = terminal.backend().buffer();
                    let line = |y| {
                        let mut text = String::new();
                        let mut x = 0;
                        while x < width {
                            let symbol = buffer[(x, y)].symbol();
                            text.push_str(symbol);
                            x += unicode_width::UnicodeWidthStr::width(symbol).max(1) as u16;
                        }
                        text
                    };
                    assert!(line(0).starts_with(&format!("┌{title}")));
                    assert!(line(row).contains(&options.to_string()));
                    assert!(!line(1 - row).contains(&options.to_string()));
                    assert_eq!(table, Rect::new(2, row + 1, width - 3, 6 - row));
                    let x = if row == 0 {
                        width - 2 - options.width() as u16
                    } else {
                        2
                    };
                    assert_eq!(buffer[(x, row)].fg, MUTED);
                    assert_eq!(
                        buffer[(x + options.spans[0].width() as u16, row)].fg,
                        ACCENT
                    );
                }
            }
        }
    }
}
