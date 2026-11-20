//! Repository list filter and sort fields.
use super::*;
use crate::{pointer, repository_list::Menu};

fn options(filter: Filter, order: Order) -> [list_heading::Field; 2] {
    [
        list_heading::Field::new(
            i18n::tr!(RepositoryFilterLabel),
            filter.label(),
            pointer::Action::RepositoryMenu(Menu::Filter),
        ),
        list_heading::Field::new(
            i18n::tr!(RepositorySortLabel),
            order.label(),
            pointer::Action::RepositoryMenu(Menu::Sort),
        ),
    ]
}

pub(super) fn render(
    frame: &mut Frame,
    title: String,
    filter: Filter,
    order: Order,
    area: Rect,
    pointer: &mut pointer::Map,
) -> Rect {
    list_heading::render(
        frame,
        &title,
        ratatui::widgets::Padding::left(1),
        &options(filter, order),
        area,
        pointer,
    )
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
                let options = list_heading::line(&options(filter, order));
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
