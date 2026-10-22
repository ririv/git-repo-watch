//! Scoped wheel navigation through the existing keyboard transitions.
use super::super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Position;
impl Screen {
    pub(crate) fn mouse(&mut self, event: MouseEvent, root: &Cancel) -> Result<bool> {
        self.view_revision += 1;
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
                | Mode::Branches(_)
                | Mode::Files(_)
                | Mode::Diff { .. }
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
