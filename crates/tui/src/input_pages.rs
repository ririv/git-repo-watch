//! Finite page sets keep each bottom action bar within three modes.
mod typing;
/// Views declare an available action; only this policy decides the initial page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Availability<A> {
    InputOnly,
    Actions(A),
}

/// A selected page survives refresh until an explicit transition or a new scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Selection<A, R = A> {
    Input,
    Typing(R),
    Actions(A),
}
impl<A, R> Selection<A, R> {
    pub fn new(available: Availability<A>) -> Self {
        match available {
            Availability::InputOnly => Self::Input,
            Availability::Actions(action) => Self::Actions(action),
        }
    }
    pub fn page(&self) -> Pair {
        match self {
            Self::Input | Self::Typing(_) => Pair::Content,
            Self::Actions(_) => Pair::Actions,
        }
    }
    pub fn is_input(&self) -> bool {
        matches!(self, Self::Input | Self::Typing(_))
    }
    pub fn take_actions(&mut self) -> Option<A> {
        match std::mem::replace(self, Self::Input) {
            Self::Actions(action) => Some(action),
            other => {
                *self = other;
                None
            }
        }
    }
    pub fn begin_typing<E>(&mut self, split: impl FnOnce(A) -> (R, E)) -> Option<E> {
        let action = self.take_actions()?;
        let (resume, effect) = split(action);
        *self = Self::Typing(resume);
        Some(effect)
    }
    pub fn restore_empty<E>(&mut self, empty: bool, join: impl FnOnce(R) -> (A, E)) -> Option<E> {
        if !empty || !matches!(self, Self::Typing(_)) {
            return None;
        }
        let Self::Typing(resume) = std::mem::replace(self, Self::Input) else {
            unreachable!()
        };
        let (action, effect) = join(resume);
        *self = Self::Actions(action);
        Some(effect)
    }
    pub fn toggle(&mut self, available: Availability<A>) {
        let previous = std::mem::replace(self, Self::Input);
        if !matches!(previous, Self::Actions(_)) {
            *self = match available {
                Availability::InputOnly => Self::Input,
                Availability::Actions(action) => Self::Actions(action),
            };
        }
    }
}
impl<A> Selection<A> {
    pub fn start_typing(&mut self) {
        self.begin_typing(|action| (action, ()));
    }
    pub fn finish_empty(&mut self, empty: bool) {
        self.restore_empty(empty, |action| (action, ()));
    }
    pub fn into_value(self, input: A) -> A {
        match self {
            Self::Input => input,
            Self::Typing(action) | Self::Actions(action) => action,
        }
    }
}

/// Two-page editors share the same cycle and page numbering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pair {
    Content,
    Actions,
}
impl Pair {
    pub fn next(self) -> Self {
        match self {
            Self::Content => Self::Actions,
            Self::Actions => Self::Content,
        }
    }
    pub fn position(self) -> (usize, usize) {
        (
            match self {
                Self::Content => 1,
                Self::Actions => 2,
            },
            2,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Input,
    Conflict,
    Replay,
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum Pages {
    Input,
    Conflict,
    Replay,
    ConflictReplay,
}
impl Pages {
    pub fn all(self) -> &'static [Page] {
        use Page::*;
        match self {
            Self::Input => &[Input],
            Self::Conflict => &[Input, Conflict],
            Self::Replay => &[Input, Replay],
            Self::ConflictReplay => &[Input, Conflict, Replay],
        }
    }
    pub fn position(self, page: Page) -> Option<(usize, usize)> {
        self.all()
            .iter()
            .position(|value| *value == page)
            .map(|index| (index + 1, self.all().len()))
    }
    pub fn next(self, page: Page) -> Page {
        let index = self
            .all()
            .iter()
            .position(|value| *value == page)
            .expect("active input page belongs to this view");
        self.all()[(index + 1) % self.all().len()]
    }
}

/// A manual choice belongs to one visible workflow, not to unrelated later views.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Scene {
    List,
    Conflict(std::path::PathBuf),
    Recovery {
        kind: engine::operations::replay::Kind,
        commit: engine::local::history::Oid,
    },
}
impl Scene {
    fn available(&self) -> Availability<Page> {
        match self {
            Self::List => Availability::InputOnly,
            Self::Conflict(_) => Availability::Actions(Page::Conflict),
            Self::Recovery { .. } => Availability::Actions(Page::Replay),
        }
    }
}
/// A temporary text edit can return only to an action page.
#[derive(Debug, Clone)]
pub(crate) enum ActionPage {
    Conflict(crate::conflict::controls::Action),
    Replay(engine::operations::replay::Control),
}
impl ActionPage {
    pub fn page(&self) -> Page {
        match self {
            Self::Conflict(_) => Page::Conflict,
            Self::Replay(_) => Page::Replay,
        }
    }
}
#[derive(Debug, Default, Clone)]
pub(crate) enum Preference {
    #[default]
    Automatic,
    Manual {
        scene: Scene,
        page: Page,
    },
    Typing {
        scene: Scene,
        resume: ActionPage,
    },
}
impl Preference {
    pub fn resolve(&mut self, scene: &Scene, current: Page) -> Page {
        match self {
            Self::Typing { .. } => current,
            Self::Manual {
                scene: previous,
                page,
            } if previous == scene => *page,
            _ => {
                *self = Self::Automatic;
                Selection::new(scene.available()).into_value(Page::Input)
            }
        }
    }
}

impl crate::Screen {
    pub(crate) fn footer_page_number(&self) -> Option<(usize, usize)> {
        match &self.mode {
            crate::Mode::Management(session)
                if matches!(session.source, crate::management::Source::Scan { .. }) =>
            {
                Some(session.focus.page().position())
            }
            crate::Mode::Workflow(session) => Some(session.page().position()),
            crate::Mode::CommitMessage {
                editor: crate::commit_form::Draft::Editing { focus, .. },
                ..
            } => Some(focus.page().position()),
            crate::Mode::StashSave(session) => Some(session.focus.page().position()),
            crate::Mode::History(view)
                if matches!(view.as_ref(), crate::history::View::Graph(_))
                    && view.session().selection.active()
                    && !view.session().picks().is_empty() =>
            {
                Some((
                    if view.session().selection.menu().is_some() {
                        2
                    } else {
                        1
                    },
                    2,
                ))
            }
            crate::Mode::Files(_) | crate::Mode::Changes(_) => {
                let pages = self.input_pages();
                if pages.all().len() > 1 {
                    pages.position(self.input_page())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declared_page_sets_cycle_once_with_accurate_positions() {
        for pages in [
            Pages::Input,
            Pages::Conflict,
            Pages::Replay,
            Pages::ConflictReplay,
        ] {
            let mut current = Page::Input;
            for (index, expected) in pages.all().iter().copied().enumerate() {
                assert_eq!(current, expected);
                assert_eq!(
                    pages.position(current),
                    Some((index + 1, pages.all().len()))
                );
                current = pages.next(current);
            }
            assert_eq!(current, Page::Input);
        }
    }
}
