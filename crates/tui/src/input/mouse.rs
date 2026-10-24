//! Scoped wheel navigation through the existing keyboard transitions.
use super::super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Position;
impl Screen {
    pub(crate) fn mouse(&mut self, event: MouseEvent, root: &Cancel) -> Result<bool> {
        if self.config.interaction_mode == engine::state::InteractionMode::TextSelection {
            return Ok(false);
        }
        if matches!(event.kind, MouseEventKind::Up(_)) {
            return Ok(self.drag.take().is_some());
        }
        if matches!(
            event.kind,
            MouseEventKind::Drag(crossterm::event::MouseButton::Left)
        ) {
            self.clicks.reset();
            if !self.config.mouse.precise_click || !matches!(self.bar, bar::InputBar::Search(_)) {
                self.drag = None;
                return Ok(false);
            }
            let Some(grab) = self.drag.as_ref() else {
                return Ok(false);
            };
            if !grab.valid(&self.pointer) {
                self.drag = None;
                return Ok(false);
            }
            let action = grab.action(Position::new(event.column, event.row));
            self.scroll_input.keyboard(std::time::Instant::now());
            self.view_revision += 1;
            return Ok(action.is_some_and(|action| self.precise_action(action)));
        }
        if matches!(
            event.kind,
            MouseEventKind::Down(_)
                | MouseEventKind::ScrollUp
                | MouseEventKind::ScrollDown
                | MouseEventKind::ScrollLeft
                | MouseEventKind::ScrollRight
        ) {
            self.drag = None;
        }
        let click = event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left);
        let wheel = matches!(
            event.kind,
            MouseEventKind::ScrollUp
                | MouseEventKind::ScrollDown
                | MouseEventKind::ScrollLeft
                | MouseEventKind::ScrollRight
        );
        if wheel
            || matches!(
                event.kind,
                MouseEventKind::Drag(_)
                    | MouseEventKind::Down(
                        crossterm::event::MouseButton::Right
                            | crossterm::event::MouseButton::Middle
                    )
            )
        {
            self.clicks.reset();
        }
        if (!click && !wheel) || (wheel && !self.config.mouse.scroll) {
            return Ok(false);
        }
        self.view_revision += 1;
        let precise = self.config.mouse.precise_click;
        if precise
            && click
            && matches!(self.bar, bar::InputBar::Search(_))
            && let Some(target) = self
                .pointer
                .links
                .hit(Position::new(event.column, event.row))
        {
            self.clicks.reset();
            self.scroll_input.keyboard(std::time::Instant::now());
            self.message = match self.link_opener.submit(target) {
                Ok(()) => i18n::tr!(LinkOpening).into(),
                Err(error) => format!("{}: {error:#}", i18n::tr!(LinkOpenFailed)),
            };
            return Ok(true);
        }
        if precise
            && matches!(self.bar, bar::InputBar::Search(_))
            && event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left)
            && let Some(destination) = self
                .breadcrumb_items
                .iter()
                .find(|(area, _)| area.contains(Position::new(event.column, event.row)))
                .map(|(_, destination)| *destination)
        {
            self.clicks.reset();
            self.follow_breadcrumb(destination);
            return Ok(true);
        }
        if matches!(self.bar, bar::InputBar::Layout { .. }) {
            if precise
                && event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left)
                && let Some(input) = self.input_area
            {
                let inner = crate::view::input::content_area(input);
                let choice = crate::view::layout_picker::options(inner, self.diff_preference)
                    .into_iter()
                    .find(|(area, _, _)| area.contains(Position::new(event.column, event.row)))
                    .map(|(_, preference, _)| preference);
                if let Some(choice) = choice
                    && let bar::InputBar::Layout { selected, .. } = &mut self.bar
                {
                    *selected = choice;
                    if self.clicks.press(
                        super::clicks::Identity::Layout(choice),
                        Position::new(event.column, event.row),
                        std::time::Instant::now(),
                    ) {
                        self.dispatch_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), root)?;
                    }
                    return Ok(true);
                }
            }
            self.clicks.reset();
            return Ok(false);
        }
        if precise
            && event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left)
            && let Some(index) = self
                .command_items
                .iter()
                .find(|(area, _)| area.contains(Position::new(event.column, event.row)))
                .map(|(_, index)| *index)
            && let bar::InputBar::Command { selected, .. } = &mut self.bar
        {
            self.clicks.reset();
            *selected = index;
            if self
                .bar
                .command_input()
                .is_some_and(|input| input.prefix_selected())
            {
                let command = self.bar.commands().expect("command candidates")[index];
                if let bar::InputBar::Command {
                    input, selected, ..
                } = &mut self.bar
                {
                    input.complete(command);
                    *selected = 0;
                }
            }
            self.dispatch_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), root)?;
            return Ok(true);
        }
        if click {
            if !matches!(self.bar, bar::InputBar::Search(_)) {
                self.clicks.reset();
                return Ok(false);
            }
            let hit = precise
                .then(|| self.pointer.hit(Position::new(event.column, event.row)))
                .flatten();
            let before = crate::workspace::Section::focused(&self.mode);
            let browser_focus_before = match &self.mode {
                Mode::Browser(session) => Some((session.focus, session.presentation().active())),
                _ => None,
            };
            let focus_before = match &self.mode {
                Mode::Changes(session) => Some(session.focus),
                _ => None,
            };
            if !self.pointer_focus(event) {
                self.clicks.reset();
                return Ok(false);
            }
            if let Some(hit) = hit {
                self.scroll_input.keyboard(std::time::Instant::now());
                if matches!(hit, crate::pointer::Hit::Scrollbar(_)) {
                    self.drag = self.pointer.grab(Position::new(event.column, event.row));
                    if self.drag.is_some() {
                        self.clicks.reset();
                        return Ok(true);
                    }
                }
                let applied = self.precise_action(hit.action());
                if applied
                    && let crate::pointer::Hit::Item(target, _) = hit
                    && let Some(identity) = self.clicked_identity(target)
                {
                    let paired = self.clicks.press(
                        identity,
                        Position::new(event.column, event.row),
                        std::time::Instant::now(),
                    );
                    // 目录在首击时开合，忽略双击的第二下，避免立即反向切换。
                    let activate = if matches!(
                        (target, &self.mode),
                        (crate::pointer::Target::Browser, Mode::Browser(session))
                            if session.selected().is_some_and(|entry| entry.directory())
                    ) {
                        !paired
                    } else {
                        paired
                    };
                    if activate {
                        self.dispatch_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), root)?;
                    }
                } else {
                    self.clicks.reset();
                }
                return Ok(applied);
            }
            self.clicks.reset();
            return Ok(before != crate::workspace::Section::focused(&self.mode)
                || focus_before
                    != match &self.mode {
                        Mode::Changes(session) => Some(session.focus),
                        _ => None,
                    }
                || browser_focus_before
                    != match &self.mode {
                        Mode::Browser(session) => {
                            Some((session.focus, session.presentation().active()))
                        }
                        _ => None,
                    });
        }
        let horizontal = match event.kind {
            MouseEventKind::ScrollLeft => Some(KeyCode::Left),
            MouseEventKind::ScrollRight => Some(KeyCode::Right),
            MouseEventKind::ScrollUp if event.modifiers.contains(KeyModifiers::SHIFT) => {
                Some(KeyCode::Left)
            }
            MouseEventKind::ScrollDown if event.modifiers.contains(KeyModifiers::SHIFT) => {
                Some(KeyCode::Right)
            }
            _ => None,
        };
        if let Some(code) = horizontal {
            if !matches!(self.bar, bar::InputBar::Search(_))
                || !self.bar.editor().text.is_empty()
                || !self
                    .body_area
                    .is_some_and(|area| area.contains(Position::new(event.column, event.row)))
            {
                return Ok(false);
            }
            if !self.pointer_focus(event) {
                return Ok(false);
            }
            return Ok(self.horizontal_key(KeyEvent::new(code, KeyModifiers::NONE)));
        }
        let code = match event.kind {
            MouseEventKind::ScrollUp => KeyCode::Up,
            MouseEventKind::ScrollDown => KeyCode::Down,
            _ => return Ok(false),
        };
        if !matches!(
            self.mode,
            Mode::Browsing
                | Mode::Browser(_)
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
        if matches!(self.bar, bar::InputBar::Search(_)) && !self.pointer_focus(event) {
            return Ok(false);
        }
        for _ in 0..3 {
            self.dispatch_key(KeyEvent::new(code, KeyModifiers::NONE), root)?;
        }
        Ok(true)
    }
    fn pointer_focus(&mut self, event: MouseEvent) -> bool {
        let point = Position::new(event.column, event.row);
        if let Mode::Browser(session) = &mut self.mode {
            let Some(focus) = self.body_area.and_then(|area| {
                crate::view::browser::areas(area, session)
                    .into_iter()
                    .find(|(area, _)| area.contains(point))
                    .map(|(_, focus)| focus)
            }) else {
                return false;
            };
            if focus != session.focus {
                if !self.config.mouse.select_pane {
                    return false;
                }
                session.focus = focus;
            }
            if focus == crate::browser::Focus::Preview
                && let Some(area) = self.body_area.and_then(|area| {
                    crate::view::browser::areas(area, session)
                        .into_iter()
                        .find(|(_, focus)| *focus == crate::browser::Focus::Preview)
                        .map(|(area, _)| area)
                })
                && let Some(mode) = crate::view::browser::preview_mode_at(area, session, point)
                && mode != session.presentation().active()
            {
                if !self.config.mouse.select_pane {
                    return false;
                }
                session.focus_mode(mode);
            }
        } else if let Mode::Changes(session) = &mut self.mode {
            let panel = self.body_area.and_then(|area| {
                crate::view::changes::areas(area, session)
                    .into_iter()
                    .find(|(area, _)| area.contains(point))
                    .map(|(_, focus)| focus)
            });
            let Some(focus) = panel else {
                return false;
            };
            if focus != session.focus {
                if !self.config.mouse.select_pane {
                    return false;
                }
                session.focus = focus;
            }
        } else if self.page == Page::Detail && crate::workspace::Workspace::is_root(&self.mode) {
            let section = self
                .workspace_areas
                .iter()
                .find(|(area, _)| area.contains(point))
                .map(|(_, section)| *section);
            let Some(section) = section else {
                return false;
            };
            if crate::workspace::Section::focused(&self.mode) != Some(section) {
                if !self.config.mouse.select_pane {
                    return false;
                }
                self.switch_repository_section(section);
            }
        }
        true
    }
}
