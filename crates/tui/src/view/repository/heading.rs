//! Responsive repository panel titles and list options.
use super::*;

enum Placement {
    Title,
    Content,
}

fn options(filter: Filter, order: Order) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            i18n::tf!(FieldLabel, label = i18n::tr!(RepositoryFilterLabel)),
            Style::default().fg(MUTED),
        ),
        Span::styled(filter.label(), Style::default().fg(ACCENT)),
        Span::styled(" · ", Style::default().fg(MUTED)),
        Span::styled(
            i18n::tf!(FieldLabel, label = i18n::tr!(RepositorySortLabel)),
            Style::default().fg(MUTED),
        ),
        Span::styled(order.label(), Style::default().fg(ACCENT)),
    ])
}

pub(super) fn render(
    frame: &mut Frame,
    title: String,
    filter: Filter,
    order: Order,
    area: Rect,
) -> Rect {
    let mut options = options(filter, order);
    // 按面板实际列宽计算，标题之间留一格边框，选项两侧各留一格空白。
    let needed = Line::from(title.as_str()).width() + options.width() + 3;
    let placement = if needed <= usize::from(area.width.saturating_sub(2)) {
        Placement::Title
    } else {
        Placement::Content
    };
    let border = block(title).padding(ratatui::widgets::Padding::left(1));
    let content = border.inner(area);
    match placement {
        Placement::Title => {
            options.spans.insert(0, Span::raw(" "));
            options.spans.push(Span::raw(" "));
            frame.render_widget(border.title(options.right_aligned()), area);
            content
        }
        Placement::Content => {
            frame.render_widget(border, area);
            let parts =
                Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(content);
            frame.render_widget(Paragraph::new(options), parts[0]);
            parts[1]
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
                let options = options(filter, order);
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
