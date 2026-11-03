//! Candidate panels share the input border and optionally label parameter choices.
use super::*;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Source {
    Commands(Vec<&'static str>),
    Options(Vec<command::options::Value>),
    Directories(Vec<String>),
    Values(Vec<String>),
}
impl Source {
    fn len(&self) -> usize {
        match self {
            Self::Commands(items) => items.len(),
            Self::Options(items) => items.len(),
            Self::Directories(items) | Self::Values(items) => items.len(),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Viewport(State);
#[derive(Debug, Default)]
enum State {
    #[default]
    Closed,
    Open {
        source: Source,
        list: ListState,
    },
}
impl Viewport {
    fn select(&mut self, source: Source, selected: Option<usize>) -> &mut ListState {
        // 只有候选内容或菜单类型变化时重置视口；方向键移动保留已有偏移。
        if !matches!(&self.0, State::Open { source: current, .. } if *current == source) {
            self.0 = State::Open {
                source,
                list: ListState::default(),
            };
        }
        let State::Open { list, .. } = &mut self.0 else {
            unreachable!("menu is open")
        };
        list.select(selected);
        list
    }
}

pub(super) fn render_list(
    frame: &mut Frame,
    areas: (Rect, Rect),
    items: Vec<ListItem<'static>>,
    selected: Option<usize>,
    source: Source,
    viewport: &mut Viewport,
) -> Vec<(Rect, usize)> {
    let (area, inner) = areas;
    let count = source.len();
    let state = viewport.select(source, selected.filter(|index| *index < count));
    frame.render_stateful_widget(
        List::new(items).highlight_style(selected_style()),
        inner,
        state,
    );
    crate::scrollbar::vertical(frame, area, inner, count, state.offset());
    (state.offset()..count)
        .take(inner.height.into())
        .enumerate()
        .map(|(row, index)| {
            (
                Rect::new(inner.x, inner.y + row as u16, inner.width, 1),
                index,
            )
        })
        .collect()
}

pub(super) fn panel(
    frame: &mut Frame,
    screen: &Screen,
    body: Rect,
    count: usize,
    title: Option<&str>,
) -> (Rect, Rect) {
    let input = screen.input_area.filter(|input| {
        input.height >= 3
            && input.width >= 2
            && input.x == body.x
            && input.width == body.width
            && input.y == body.bottom()
            && body.height >= 2
    });
    let available = body.height.saturating_sub(u16::from(body.height >= 9));
    let height = (count.clamp(1, 8) as u16 + if input.is_some() { 1 } else { 2 }).min(available);
    let area = Rect::new(
        body.x,
        body.bottom().saturating_sub(height),
        body.width,
        height,
    );
    let border = match input {
        Some(input) => {
            // 菜单不画底边，与输入框共用一条分隔线，避免双边框和标题重复。
            input::connect_menu(frame, screen, input);
            Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(input::active_accent(screen)))
        }
        None if height >= 3 => Block::bordered().border_style(Style::default().fg(BORDER)),
        None => Block::default(),
    };
    let border = match title {
        Some(title) if input.is_some() || height >= 3 => border
            .title(format!(" {title} "))
            .title_style(Style::default().fg(input::active_accent(screen))),
        _ => border,
    };
    let inner = border.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(border, area);
    (area, inner)
}
