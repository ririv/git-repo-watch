//! One changes workspace composes file selection and a versioned background preview.
mod discard;
pub(crate) mod edit;
mod load;
mod partial;
use crate::{FileSession, Mode, Screen, Service, document, spinner, workspace::Arrangement};
use engine::local::files::{Change, Files};
use std::{sync::Arc, time::Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Key {
    revision: u64,
    change: Change,
    theme: highlight::Theme,
}
#[derive(Debug)]
pub(super) enum Preview {
    Empty,
    Directory {
        path: std::path::PathBuf,
        count: usize,
    },
    Loading(Key),
    Ready {
        key: Key,
        content: Box<document::View>,
        scroll: usize,
        editing: edit::State,
    },
    Failed {
        key: Key,
        error: String,
    },
}
impl Preview {
    fn key(&self) -> Option<&Key> {
        match self {
            Self::Empty | Self::Directory { .. } => None,
            Self::Loading(key) | Self::Ready { key, .. } | Self::Failed { key, .. } => Some(key),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Focus {
    Files,
    Preview,
}
#[derive(Debug, Clone, Copy)]
pub(super) enum ReturnTo {
    Repositories,
    Files,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectionKey {
    preview: Key,
    lines: std::collections::BTreeSet<engine::local::files::partial::LineId>,
}

#[derive(Debug)]
pub(super) struct Session {
    pub files: FileSession,
    pub return_to: ReturnTo,
    pub focus: Focus,
    pub arrangement: Arrangement,
    pub preview: Preview,
    revision: u64,
    loader: load::Loader,
    animation: spinner::Spinner,
    pub feedback: crate::loading::Feedback<Box<Preview>>,
    bookmark: Option<Bookmark>,
}
#[derive(Debug)]
struct Bookmark {
    path: std::path::PathBuf,
    selection: Option<edit::Bookmark>,
    reading: Option<crate::document::Position>,
}
impl Session {
    pub fn previews_file_diff(&self) -> bool {
        self.preview.key().is_some_and(|key| key.change.kind != 'U')
    }

    pub fn selection_key(&self) -> Option<SelectionKey> {
        match &self.preview {
            Preview::Ready {
                key,
                editing: edit::State::Ready(editing),
                ..
            } => Some(SelectionKey {
                preview: key.clone(),
                lines: editing.selected(),
            }),
            _ => None,
        }
    }

    pub fn new(files: FileSession, return_to: ReturnTo) -> Self {
        Self {
            files,
            return_to,
            focus: Focus::Files,
            arrangement: Arrangement::Tiled,
            preview: Preview::Empty,
            revision: 0,
            loader: Default::default(),
            animation: spinner::Spinner::new(Instant::now()),
            feedback: Default::default(),
            bookmark: None,
        }
    }
    pub fn refreshed(&mut self, snapshot: Files) {
        self.bookmark = match &self.preview {
            Preview::Ready {
                key,
                editing,
                content,
                scroll,
                ..
            } => Some(Bookmark {
                path: key.change.location.name().to_owned(),
                selection: match editing {
                    edit::State::Ready(editing) => Some(editing.bookmark()),
                    _ => None,
                },
                reading: content.position(*scroll),
            }),
            Preview::Loading(_) => self.bookmark.take(),
            _ => None,
        };
        self.files = self.files.refreshed(snapshot);
        self.revision += 1;
    }
    pub fn pending(&self) -> bool {
        self.loader.pending()
    }
    pub fn symbol(&self) -> &'static str {
        self.animation.symbol()
    }
    #[cfg(test)]
    pub fn poll_at(&mut self, service: &Arc<Service>, now: Instant) -> bool {
        self.poll_at_with_theme(service, highlight::Theme::Dark, now)
    }
    pub fn poll_with_theme(&mut self, service: &Arc<Service>, theme: highlight::Theme) -> bool {
        self.poll_at_with_theme(service, theme, Instant::now())
    }
    fn poll_at_with_theme(
        &mut self,
        service: &Arc<Service>,
        theme: highlight::Theme,
        now: Instant,
    ) -> bool {
        if self.preview.key().is_some_and(|key| key.theme != theme) {
            self.refreshed(self.files.snapshot.clone());
        }
        if let crate::files::Cursor::Directory { path, .. } = &self.files.cursor {
            let count = self.files.focused_changes().len();
            let changed = !matches!(&self.preview, Preview::Directory { path: current, count: current_count } if current == path && *current_count == count);
            self.loader.clear();
            // 目录预览不发起读取，但仍须回收已取消的文件读取任务。
            let _ = self.loader.poll(service);
            self.feedback.finish();
            if changed {
                self.preview = Preview::Directory {
                    path: path.clone(),
                    count,
                };
            }
            return changed;
        }
        let next = self.files.current().map(|change| Key {
            revision: self.revision,
            change: change.clone(),
            theme,
        });
        let mut changed = false;
        if self.preview.key() != next.as_ref() || matches!(self.preview, Preview::Directory { .. })
        {
            match &next {
                Some(key) => {
                    self.loader
                        .request(key.clone(), self.files.snapshot.clone(), service);
                    let previous =
                        std::mem::replace(&mut self.preview, Preview::Loading(key.clone()));
                    self.feedback.begin(
                        (!matches!(previous, Preview::Empty | Preview::Loading(_)))
                            .then(|| Box::new(previous)),
                        now,
                    );
                }
                None => {
                    self.loader.clear();
                    self.preview = Preview::Empty;
                    self.feedback.finish();
                }
            }
            changed = true;
        }
        if let Some((key, result)) = self.loader.poll(service)
            && self.preview.key() == Some(&key)
        {
            self.preview = match result {
                Ok(mut review) => {
                    let mut scroll = 0;
                    if let Some(bookmark) = self.bookmark.take()
                        && key.change.location.name() == bookmark.path
                    {
                        if let (Some(selection), edit::State::Ready(editing)) =
                            (bookmark.selection, &mut review.editing)
                        {
                            editing.restore(&selection);
                        }
                        if let Some(reading) = bookmark.reading {
                            review.content.restore_position(reading, &mut scroll);
                        }
                    }
                    Preview::Ready {
                        key,
                        content: review.content,
                        editing: review.editing,
                        scroll,
                    }
                }
                Err(error) => Preview::Failed { key, error },
            };
            self.feedback.finish();
            changed = true;
        }
        if self.pending() {
            changed |= self.animation.tick(now);
        }
        if matches!(self.preview, Preview::Loading(_)) {
            changed |= self.feedback.tick(now);
        }
        changed
    }
    #[cfg(test)]
    pub fn content(&self) -> &document::View {
        match &self.preview {
            Preview::Ready { content, .. } => content,
            other => panic!("preview is not ready: {other:?}"),
        }
    }
}
impl Screen {
    pub(super) fn poll_changes(&mut self) -> bool {
        match &mut self.mode {
            Mode::Changes(session) => {
                session.files.set_layout(self.file_layout);
                session.poll_with_theme(&self.service, self.background.syntax_theme())
            }
            _ => false,
        }
    }
}
