//! Scoped wheel navigation through the existing keyboard transitions.
use super::super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Position;
impl Screen {
    pub(crate) fn mouse(&mut self, event: MouseEvent, root: &Cancel) -> Result<bool> {
        let reset = if matches!(event.kind, MouseEventKind::Moved | MouseEventKind::Up(_)) {
            false
        } else {
            self.exit_sequence.reset()
        };
        let result = self.pointer_event(event, root);
        self.sync_plain_bar();
        result.map(|changed| changed || reset)
    }
    fn pointer_event(&mut self, event: MouseEvent, root: &Cancel) -> Result<bool> {
        if self.config.interaction_mode == engine::state::InteractionMode::TextSelection {
            return Ok(false);
        }
        if self.tasks_mouse(event) {
            self.clicks.reset();
            self.drag = None;
            return Ok(true);
        }
        if matches!(event.kind, MouseEventKind::Up(_)) {
            return Ok(self.drag.take().is_some());
        }
        if matches!(
            event.kind,
            MouseEventKind::Drag(crossterm::event::MouseButton::Left)
        ) {
            self.clicks.reset();
            if !self.config.mouse.precise_click || !self.bar.is_plain() {
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
        if wheel && self.history_actions().is_some() {
            let code = match event.kind {
                MouseEventKind::ScrollUp => KeyCode::Up,
                MouseEventKind::ScrollDown => KeyCode::Down,
                _ => return Ok(false),
            };
            self.history_actions_key(KeyEvent::new(code, KeyModifiers::NONE));
            return Ok(true);
        }
        let precise = self.config.mouse.precise_click;
        if self.bar.is_plain()
            && let Mode::CommitMessage {
                plan,
                editor,
                scroll,
            } = &mut self.mode
        {
            let point = Position::new(event.column, event.row);
            if click
                && precise
                && let Some(input) = self.input_area
                && let Some((_, action)) = crate::view::commit::options(
                    crate::view::input::content_area(input),
                    editor,
                    self.config.ai.enabled,
                )
                .into_iter()
                .find(|(area, _)| area.contains(point))
            {
                editor.select(action);
                let execute = self.clicks.press(
                    super::clicks::Identity::CommitAction {
                        repository: plan.path.clone(),
                        action,
                    },
                    point,
                    std::time::Instant::now(),
                );
                if execute {
                    self.commit_form_action(action);
                }
                return Ok(true);
            }
            if let Some(body) = self.body_area {
                let (message, review) = crate::view::commit::areas(body);
                if click
                    && precise
                    && message
                        .inner(ratatui::layout::Margin::new(2, 1))
                        .contains(point)
                {
                    editor.focus(crate::commit_form::Focus::Message);
                    return Ok(true);
                }
                if wheel
                    && matches!(
                        event.kind,
                        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                    )
                    && (message.contains(point) || review.contains(point))
                {
                    let offset = if message.contains(point) {
                        &mut editor.display().scroll
                    } else {
                        scroll
                    };
                    *offset = if event.kind == MouseEventKind::ScrollUp {
                        offset.saturating_sub(3)
                    } else {
                        offset.saturating_add(3)
                    };
                    return Ok(true);
                }
            }
        }

        if precise
            && click
            && self.bar.is_plain()
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
            && self.bar.is_plain()
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
            && let bar::InputBar::Command { selected, menu, .. } = &mut self.bar
        {
            self.clicks.reset();
            match menu {
                command::completion::Menu::Options(options) => options.selected = index,
                command::completion::Menu::Directories(directories) => {
                    if !directories.select(index) {
                        return Ok(false);
                    }
                }
                command::completion::Menu::Commands => *selected = index,
                command::completion::Menu::Values(values) => {
                    if !values.select(index) {
                        return Ok(false);
                    }
                }
                command::completion::Menu::Loading(_) | command::completion::Menu::Failed(_) => {
                    return Ok(false);
                }
            }
            if self
                .bar
                .command_input()
                .is_some_and(|input| input.prefix_selected())
            {
                let command = self.search_commands().expect("command candidates")[index];
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
            if !self.bar.is_plain() {
                self.clicks.reset();
                return Ok(false);
            }
            let hit = precise
                .then(|| self.pointer.hit(Position::new(event.column, event.row)))
                .flatten();
            if let Some(crate::pointer::Hit::Button(
                action @ crate::pointer::Action::BrowserDirectory(_),
            )) = hit
            {
                self.clicks.reset();
                return Ok(self.precise_action(action));
            }
            let before = crate::workspace::Section::focused(&self.mode);
            let browser_focus_before = match &self.mode {
                Mode::Browser(session) => Some((session.focus, session.presentation().active())),
                _ => None,
            };
            let focus_before = match &self.mode {
                Mode::Changes(session) => Some(session.focus),
                _ => None,
            };
            let history_focus_before = match &self.mode {
                Mode::History(view) => match view.as_ref() {
                    crate::history::View::Commit(commit) => Some(commit.focus),
                    _ => None,
                },
                _ => None,
            };
            if self.history_actions().is_none() && !self.pointer_focus(event) {
                self.clicks.reset();
                return Ok(false);
            }
            if let Some(hit) = hit {
                if matches!(hit, crate::pointer::Hit::Button(_)) {
                    self.clicks.reset();
                    self.drag = None;
                }
                if let crate::pointer::Hit::Item(crate::pointer::Target::ScheduleButton, index) =
                    hit
                {
                    self.clicks.reset();
                    if self.page != Page::Configuration(Configuration::Schedule)
                        || !matches!(self.mode, Mode::Browsing)
                    {
                        return Ok(false);
                    }
                    let Some(item) = Configuration::Schedule
                        .items(self.snapshot.registration, &self.config.ai)
                        .get(index)
                        .copied()
                    else {
                        return Ok(false);
                    };
                    self.setting_selected = index;
                    self.activate_setting(item, root)?;
                    return Ok(true);
                }
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
                    let directory = target == crate::pointer::Target::Files
                        && self.mode.file_session().is_some_and(|files| {
                            matches!(files.cursor, crate::files::Cursor::Directory { .. })
                        });
                    let activate = if directory
                        || matches!(
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
                || history_focus_before
                    != match &self.mode {
                        Mode::History(view) => match view.as_ref() {
                            crate::history::View::Commit(commit) => Some(commit.focus),
                            _ => None,
                        },
                        _ => None,
                    }
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
        if self.bar.is_plain()
            && let Mode::CompositionWaiting(waiting) = &mut self.mode
            && let crate::composition::waiting::State::Failed { scroll, .. } = &mut waiting.state
            && let Some(body) = self.body_area
        {
            let (_, detail) = crate::view::composition::areas(body);
            let (_, _, preview) = crate::view::composition::detail_areas(detail);
            if preview.contains(Position::new(event.column, event.row)) {
                match event.kind {
                    MouseEventKind::ScrollUp => {
                        *scroll = scroll.saturating_sub(3);
                        return Ok(true);
                    }
                    MouseEventKind::ScrollDown => {
                        *scroll = scroll.saturating_add(3);
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
        if self.bar.is_plain()
            && let Mode::Composition(session) = &mut self.mode
            && matches!(
                session.state,
                crate::composition::State::Browsing | crate::composition::State::Editing(_)
            )
            && let Some(body) = self.body_area
        {
            let (_, detail) = crate::view::composition::areas(body);
            let (message, _, preview) = crate::view::composition::detail_areas(detail);
            let point = Position::new(event.column, event.row);
            let offset = if message.contains(point) {
                Some(match &mut session.state {
                    crate::composition::State::Editing(editor) => &mut editor.scroll,
                    _ => &mut session.message_scroll,
                })
            } else if preview.contains(point) {
                Some(&mut session.scroll)
            } else {
                None
            };
            if let Some(offset) = offset {
                match event.kind {
                    MouseEventKind::ScrollUp => {
                        *offset = offset.saturating_sub(3);
                        return Ok(true);
                    }
                    MouseEventKind::ScrollDown => {
                        *offset = offset.saturating_add(3);
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
        if self.bar.is_plain()
            && let Mode::Workflow(session) = &mut self.mode
            && session
                .revealed
                .is_some_and(|(_, area)| area.contains(Position::new(event.column, event.row)))
        {
            match event.kind {
                MouseEventKind::ScrollUp => {
                    session.scroll = session.scroll.saturating_sub(3);
                    return Ok(true);
                }
                MouseEventKind::ScrollDown => {
                    session.scroll = session.scroll.saturating_add(3);
                    return Ok(true);
                }
                _ => {}
            }
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
            if !self.bar.is_plain()
                || !self
                    .body_area
                    .is_some_and(|area| area.contains(Position::new(event.column, event.row)))
            {
                return Ok(false);
            }
            if !self.pointer_focus(event) {
                return Ok(false);
            }
            return Ok(self.horizontal_scroll(if code == KeyCode::Left { -4 } else { 4 }));
        }
        let code = match event.kind {
            MouseEventKind::ScrollUp => KeyCode::Up,
            MouseEventKind::ScrollDown => KeyCode::Down,
            _ => return Ok(false),
        };
        if !matches!(
            self.mode,
            Mode::Management(_)
                | Mode::Browsing
                | Mode::Browser(_)
                | Mode::Panel(_)
                | Mode::Stashes(_)
                | Mode::Branches(_)
                | Mode::Files(_)
                | Mode::Changes(_)
                | Mode::History(_)
                | Mode::Composition(_)
        ) && !matches!(
            self.mode,
            Mode::Confirm {
                operation: Confirmation::ReplayParents { .. },
                ..
            }
        ) {
            return Ok(false);
        }
        // 主列表只在自身区域响应，避免在右侧预览和底部输入时改变选中仓库。
        let area = if matches!(self.mode, Mode::Browsing)
            && self.page == Page::Repositories
            && self.bar.is_plain()
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
        if self.bar.is_plain() && !self.pointer_focus(event) {
            return Ok(false);
        }
        if self.bar.is_plain()
            && let Mode::Changes(session) = &mut self.mode
            && session.focus == crate::changes::Focus::Preview
            && let crate::changes::Preview::Ready { scroll, .. } = &mut session.preview
        {
            *scroll = if code == KeyCode::Up {
                scroll.saturating_sub(3)
            } else {
                scroll.saturating_add(3)
            };
            return Ok(true);
        }
        for _ in 0..3 {
            self.dispatch_key(KeyEvent::new(code, KeyModifiers::NONE), root)?;
        }
        Ok(true)
    }
    fn pointer_focus(&mut self, event: MouseEvent) -> bool {
        let point = Position::new(event.column, event.row);
        if matches!(self.mode, Mode::Management(_)) {
            return self.body_area.is_some_and(|area| area.contains(point));
        }
        if let Mode::History(view) = &mut self.mode
            && let crate::history::View::Commit(commit) = view.as_mut()
        {
            let Some(focus) = self.body_area.and_then(|area| {
                crate::view::history::detail::areas(area, commit)
                    .into_iter()
                    .find(|(area, _)| area.contains(point))
                    .map(|(_, focus)| focus)
            }) else {
                return false;
            };
            if focus != commit.focus && !self.config.mouse.select_pane {
                return false;
            }
            commit.focus = focus;
            return true;
        }
        if matches!(self.mode, Mode::CompositionWaiting(_)) {
            return self.body_area.is_some_and(|area| area.contains(point));
        }
        if let Mode::Composition(session) = &mut self.mode {
            if matches!(session.state, crate::composition::State::Editing(_)) {
                return self.body_area.is_some_and(|area| {
                    let (_, detail) = crate::view::composition::areas(area);
                    let (message, _, preview) = crate::view::composition::detail_areas(detail);
                    message.contains(point) || preview.contains(point)
                });
            }
            if !matches!(session.state, crate::composition::State::Browsing) {
                return false;
            }
            let Some(area) = self.body_area else {
                return false;
            };
            let (commits, detail) = crate::view::composition::areas(area);
            let (_, units, _) = crate::view::composition::detail_areas(detail);
            let focus = if commits.contains(point) {
                crate::composition::Focus::Commits
            } else if units.contains(point) {
                crate::composition::Focus::Units
            } else {
                return area.contains(point);
            };
            if focus != session.focus && !self.config.mouse.select_pane {
                return false;
            }
            session.focus = focus;
            return true;
        }
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
