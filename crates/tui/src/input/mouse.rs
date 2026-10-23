//! Scoped wheel navigation through the existing keyboard transitions.
use super::super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Position;
impl Screen {
    pub(crate) fn mouse(&mut self, event: MouseEvent, root: &Cancel) -> Result<bool> {
        self.view_revision += 1;
        if matches!(self.bar, bar::InputBar::Layout { .. }) {
            if event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left)
                && let Some(body) = self.body_area
            {
                let inner = ratatui::layout::Rect::new(
                    body.x + 1,
                    body.bottom() + 2,
                    body.width.saturating_sub(2),
                    1,
                );
                let choice = crate::view::layout_picker::options(inner, self.diff_preference)
                    .into_iter()
                    .find(|(area, _, _)| area.contains(Position::new(event.column, event.row)))
                    .map(|(_, preference, _)| preference);
                if let Some(choice) = choice
                    && let bar::InputBar::Layout { selected, .. } = &mut self.bar
                {
                    *selected = choice;
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        if event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left)
            && let Some(index) = self
                .command_items
                .iter()
                .find(|(area, _)| area.contains(Position::new(event.column, event.row)))
                .map(|(_, index)| *index)
            && let bar::InputBar::Command { selected, .. } = &mut self.bar
        {
            *selected = index;
            self.dispatch_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), root)?;
            return Ok(true);
        }
        if let Mode::Changes(session) = &mut self.mode
            && matches!(self.bar, bar::InputBar::Search(_))
        {
            let panel = self.body_area.and_then(|area| {
                crate::view::changes::areas(area, session)
                    .into_iter()
                    .find(|(area, _)| area.contains(Position::new(event.column, event.row)))
                    .map(|(_, focus)| focus)
            });
            if let Some(focus) = panel {
                session.focus = focus;
            }
            if event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left) {
                return Ok(panel.is_some());
            }
        }
        if event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left)
            && matches!(self.bar, bar::InputBar::Search(_))
        {
            let section = self
                .workspace_areas
                .iter()
                .find(|(area, _)| area.contains(Position::new(event.column, event.row)))
                .map(|(_, section)| *section);
            if let Some(section) = section {
                self.switch_repository_section(section);
                return Ok(true);
            }
        }
        let code = match event.kind {
            MouseEventKind::ScrollUp => KeyCode::Up,
            MouseEventKind::ScrollDown => KeyCode::Down,
            _ => return Ok(false),
        };
        if !matches!(
            self.mode,
            Mode::Browsing
                | Mode::Panel(_)
                | Mode::Stashes(_)
                | Mode::Branches(_)
                | Mode::Files(_)
                | Mode::Changes(_)
                | Mode::History(_)
        ) {
            return Ok(false);
        }
        // 主列表只在自身区域响应，避免在右侧预览和底部输入时改变选中仓库。
        let area = if matches!(self.mode, Mode::Browsing)
            && self.page == Page::Repositories
            && matches!(self.bar, bar::InputBar::Search(_))
        {
            self.repository_area
        } else {
            self.body_area
        };
        if !area.is_some_and(|area| area.contains(Position::new(event.column, event.row))) {
            return Ok(false);
        }
        let direction = match code {
            KeyCode::Up => super::scroll::Direction::Up,
            KeyCode::Down => super::scroll::Direction::Down,
            _ => unreachable!(),
        };
        if !self
            .scroll_input
            .wheel(direction, std::time::Instant::now())
        {
            return Ok(false);
        }
        if self.page == Page::Detail
            && crate::workspace::Workspace::is_root(&self.mode)
            && matches!(self.bar, bar::InputBar::Search(_))
        {
            let section = self
                .workspace_areas
                .iter()
                .find(|(area, _)| area.contains(Position::new(event.column, event.row)))
                .map(|(_, section)| *section);
            if let Some(section) = section {
                self.switch_repository_section(section);
            }
        }
        for _ in 0..3 {
            self.dispatch_key(KeyEvent::new(code, KeyModifiers::NONE), root)?;
        }
        Ok(true)
    }
}
