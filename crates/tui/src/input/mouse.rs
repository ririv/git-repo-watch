//! Scoped wheel navigation through the existing keyboard transitions.
use super::super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Margin, Position},
};
use unicode_width::UnicodeWidthStr;
impl Screen {
    #[cfg(test)]
    pub(crate) fn mouse(&mut self, event: MouseEvent, root: &Cancel) -> Result<bool> {
        self.mouse_in_frame(event, root, &Buffer::empty(ratatui::layout::Rect::ZERO))
    }
    pub(crate) fn mouse_in_frame(
        &mut self,
        event: MouseEvent,
        root: &Cancel,
        rendered: &Buffer,
    ) -> Result<bool> {
        let reset = if matches!(event.kind, MouseEventKind::Moved | MouseEventKind::Up(_)) {
            false
        } else {
            self.exit_sequence.reset()
        };
        let result = self.pointer_event(event, root, rendered);
        self.sync_plain_bar();
        result.map(|changed| changed || reset)
    }
    fn pointer_event(
        &mut self,
        event: MouseEvent,
        root: &Cancel,
        rendered: &Buffer,
    ) -> Result<bool> {
        if self.config.interaction_mode == engine::state::InteractionMode::TextSelection {
            return Ok(false);
        }
        if self.tasks.session.open() && self.command_click(event, root)? {
            return Ok(true);
        }
        if self.config.mouse.precise_click
            && event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left)
            && self.active_bar().is_plain()
            && let Some(crate::pointer::Hit::Button(crate::pointer::Action::Command(action))) =
                self.pointer.hit(Position::new(event.column, event.row))
        {
            self.clicks.reset();
            self.drag = None;
            self.view_revision += 1;
            self.activate(
                action,
                self.command_repository(),
                self.copy_selection(),
                root,
            )?;
            return Ok(true);
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
        let wheel = super::scroll::is_wheel(event.kind);
        if wheel || matches!(event.kind, MouseEventKind::Down(_)) {
            self.drag = None;
        }
        let click = event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left);
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
        if wheel && self.focused_conflict_control().is_none() {
            self.dismiss_input_actions();
        }
        if wheel && self.history_actions().is_some() {
            let code = match event.kind {
                MouseEventKind::ScrollUp => KeyCode::Up,
                MouseEventKind::ScrollDown => KeyCode::Down,
                _ => return Ok(false),
            };
            self.history_actions_key(KeyEvent::new(code, KeyModifiers::NONE));
            return Ok(true);
        }
        if let Some(changed) = self.stash_save_mouse(event, root)? {
            return Ok(changed);
        }
        if wheel
            && self.bar.is_plain()
            && self
                .history_preview
                .area
                .is_some_and(|area| area.contains(Position::new(event.column, event.row)))
        {
            match event.kind {
                MouseEventKind::ScrollUp => self.history_preview.scroll(-3),
                MouseEventKind::ScrollDown => self.history_preview.scroll(3),
                _ => return Ok(false),
            }
            return Ok(true);
        }
        let precise = self.config.mouse.precise_click;
        let action_panel = matches!(&self.mode, Mode::Management(session) if matches!(session.focus, crate::management::Focus::Actions(_)));
        #[cfg(feature = "developer-scheduler")]
        let action_panel = action_panel || matches!(self.mode, Mode::Workflow(_));
        if click
            && action_panel
            && self
                .input_area
                .is_some_and(|area| area.contains(Position::new(event.column, event.row)))
        {
            if precise && let Some(hit) = self.pointer.hit(Position::new(event.column, event.row)) {
                return Ok(self.precise_action(hit.action()));
            }
            return Ok(precise);
        }
        if click
            && self.focused_conflict_control().is_some()
            && self
                .input_area
                .is_some_and(|area| area.contains(Position::new(event.column, event.row)))
        {
            if precise
                && let Some(crate::pointer::Hit::Button(crate::pointer::Action::ConflictControl(
                    action,
                ))) = self.pointer.hit(Position::new(event.column, event.row))
            {
                self.perform_conflict_control(action);
            }
            return Ok(precise);
        }

        if click
            && self.focused_replay_control().is_some()
            && self
                .input_area
                .is_some_and(|area| area.contains(Position::new(event.column, event.row)))
        {
            if precise
                && let Some(crate::pointer::Hit::Button(crate::pointer::Action::ReplayControl(
                    control,
                ))) = self.pointer.hit(Position::new(event.column, event.row))
            {
                self.preview_replay_control(control);
            }
            return Ok(precise);
        }
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
                    editor.focus(crate::commit_form::Focus::Input);
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
            self.request_external_open(target);
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
        if self.command_click(event, root)? {
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
            if let Some(crate::pointer::Hit::Button(
                action @ crate::pointer::Action::ConflictBlock { index, .. },
            )) = hit
            {
                if !self.precise_action(action) {
                    self.clicks.reset();
                    return Ok(false);
                }
                let Mode::Changes(session) = &self.mode else {
                    unreachable!();
                };
                let Some(change) = session.files.current() else {
                    return Ok(false);
                };
                let paired = self.clicks.press(
                    super::clicks::Identity::ConflictBlock {
                        repository: session.files.snapshot.path.clone(),
                        path: change.location.name().to_owned(),
                        index,
                    },
                    Position::new(event.column, event.row),
                    std::time::Instant::now(),
                );
                if paired {
                    self.dispatch_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), root)?;
                }
                return Ok(true);
            }
            let focus_before = match &self.mode {
                Mode::Changes(session) => Some((
                    session.focus,
                    match &session.preview {
                        crate::changes::Preview::Ready { content, .. } => match content.as_ref() {
                            crate::document::View::Conflict(view) => Some(view.focus()),
                            _ => None,
                        },
                        _ => None,
                    },
                )),
                _ => None,
            };
            let inspection_focus_before = self.mode.inspection().map(|inspect| inspect.focus);
            if self.history_actions().is_none() && !self.pointer_focus(event) {
                self.clicks.reset();
                return Ok(false);
            }
            if let Some(hit) = hit {
                if matches!(hit, crate::pointer::Hit::Button(_)) {
                    self.clicks.reset();
                    self.drag = None;
                }
                #[cfg(feature = "developer-scheduler")]
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
                        .items(&self.snapshot, &self.config.ai)
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
            let point = Position::new(event.column, event.row);
            if precise
                && self.focus_mode == crate::focus::State::Inactive
                && let Some(pane) = self.zoom_pane_target()
                && let Some(area) = self.zoom_pane_area(pane)
                && area.inner(Margin::new(1, 1)).contains(point)
                && blank_cell(rendered, point)
                && let Some(repository) = self.scope_path()
            {
                let identity = super::clicks::Identity::ZoomPane { repository, pane };
                if self
                    .clicks
                    .press(identity, point, std::time::Instant::now())
                {
                    self.activate(
                        Action::Zoom,
                        self.command_repository(),
                        self.copy_selection(),
                        root,
                    )?;
                    return Ok(true);
                }
            } else {
                self.clicks.reset();
            }
            return Ok(before != crate::workspace::Section::focused(&self.mode)
                || inspection_focus_before != self.mode.inspection().map(|inspect| inspect.focus)
                || focus_before
                    != match &self.mode {
                        Mode::Changes(session) => Some((
                            session.focus,
                            match &session.preview {
                                crate::changes::Preview::Ready { content, .. } => {
                                    match content.as_ref() {
                                        crate::document::View::Conflict(view) => Some(view.focus()),
                                        _ => None,
                                    }
                                }
                                _ => None,
                            },
                        )),
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
        #[cfg(feature = "developer-scheduler")]
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
                | Mode::Worktrees(_)
                | Mode::Files(_)
                | Mode::Changes(_)
                | Mode::History(_)
                | Mode::Composition(_)
        ) && !matches!(
            self.mode,
            Mode::Confirm {
                operation: Confirmation::ReplayParents { .. } | Confirmation::DiscardFile { .. },
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
        if let Some(inspect) = self.mode.inspection_mut() {
            let Some(focus) = self.body_area.and_then(|area| {
                crate::view::inspection::areas(area, inspect)
                    .into_iter()
                    .find(|(area, _)| area.contains(point))
                    .map(|(_, focus)| focus)
            }) else {
                return false;
            };
            if focus != inspect.focus && !self.config.mouse.select_pane {
                return false;
            }
            inspect.focus = focus;
            return true;
        }
        let graph_area = if self.page == Page::Detail {
            self.workspace_areas.iter().find_map(|(area, section)| {
                (*section == crate::workspace::Section::History).then_some(*area)
            })
        } else {
            self.body_area
        };
        if graph_area.is_some_and(|area| area.contains(point))
            && let Mode::History(view) = &mut self.mode
            && let crate::history::View::Graph(session) = view.as_mut()
        {
            if !self.body_area.is_some_and(|area| area.contains(point)) {
                return false;
            }
            let focus = if self
                .history_preview
                .area
                .is_some_and(|area| area.contains(point))
            {
                crate::history::GraphFocus::Files
            } else {
                crate::history::GraphFocus::Graph
            };
            if session.focus != focus && !self.config.mouse.select_pane {
                return false;
            }
            session.focus = focus;
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
            if focus == crate::changes::Focus::Preview {
                let preview = self.body_area.and_then(|area| {
                    crate::view::changes::areas(area, session)
                        .into_iter()
                        .find_map(|(area, focus)| {
                            (focus == crate::changes::Focus::Preview).then_some(area)
                        })
                });
                if let Some(area) = preview
                    && let crate::changes::Preview::Ready {
                        content, scroll, ..
                    } = &mut session.preview
                    && let crate::document::View::Conflict(view) = content.as_mut()
                    && let Some(next) = crate::view::conflict::focus_at(area, view.mode, point)
                    && next != view.focus()
                {
                    if !self.config.mouse.select_pane {
                        return false;
                    }
                    view.select(next, scroll);
                }
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
    fn command_click(&mut self, event: MouseEvent, root: &Cancel) -> Result<bool> {
        if self.config.mouse.precise_click
            && event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left)
            && let Some(index) = self
                .command_items
                .iter()
                .find(|(area, _)| area.contains(Position::new(event.column, event.row)))
                .map(|(_, index)| *index)
            && let bar::InputBar::Command { selected, menu, .. } =
                self.tasks.input_bar_mut(&mut self.bar)
        {
            self.clicks.reset();
            match menu {
                command::completion::Menu::History(history) => {
                    if !history.select(index) {
                        return Ok(false);
                    }
                }
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
                .active_bar()
                .command_input()
                .is_some_and(|input| input.prefix_selected())
            {
                let Some(candidate) = self
                    .command_candidates()
                    .and_then(|candidates| candidates.get(index).copied())
                else {
                    return Ok(false);
                };
                if let bar::InputBar::Command {
                    input, selected, ..
                } = self.tasks.input_bar_mut(&mut self.bar)
                {
                    match candidate {
                        command::named::Candidate::Command(command) => input.complete(command),
                        command::named::Candidate::Parameter(parameter) => {
                            parameter.complete(input)
                        }
                    }
                    *selected = 0;
                }
            }
            let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
            if self.tasks.session.open() {
                self.bar_key(key, root)?;
            } else {
                self.dispatch_key(key, root)?;
            }
            return Ok(true);
        }
        Ok(false)
    }
}

fn blank_cell(buffer: &Buffer, point: Position) -> bool {
    buffer.cell(point).is_some_and(|cell| {
        cell.symbol() == " " && cell.diff_option != ratatui::buffer::CellDiffOption::Skip
    })
        // 宽字符占用的后续格在缓冲区里可能仍是空格。
        && (point.x == buffer.area.x
            || buffer[Position::new(point.x - 1, point.y)].symbol().width() <= 1)
}
