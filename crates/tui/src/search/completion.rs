//! Complete a filter from the selected row using the same fields that the view searches.
use super::Mode as SearchMode;
use crate::{Editor, Mode, Screen, bar::InputBar, branches, files::FileSession, stashes};
use std::path::Path;

enum Completion {
    List { query: String, index: usize },
    Browser(String),
    Task(String),
}

fn value(query: &str, fields: impl IntoIterator<Item = String>) -> Option<String> {
    fields
        .into_iter()
        .find(|field| field.to_lowercase().contains(query) && !field.chars().any(char::is_control))
}

fn selected<T, const N: usize>(
    rows: &[T],
    index: Option<usize>,
    query: &str,
    fields: impl Fn(&T) -> [String; N],
) -> Option<Completion> {
    let index = index?;
    let completed = value(query, fields(rows.get(index)?))?;
    let needle = completed.to_lowercase();
    // 补全后的匹配项是当前结果的子集，保留原本选中的那一项。
    let index = rows[..index]
        .iter()
        .filter(|row| {
            fields(row)
                .iter()
                .any(|field| field.to_lowercase().contains(&needle))
        })
        .count();
    Some(Completion::List {
        query: completed,
        index,
    })
}

fn path_fields(path: &Path) -> [String; 2] {
    [
        path.file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned(),
        path.to_string_lossy().into_owned(),
    ]
}

fn file_completion(files: &FileSession, query: &str) -> Option<Completion> {
    selected(&files.rows(), files.cursor.index(), query, |file| {
        [file.location.name().to_string_lossy().into_owned()]
    })
}

impl Screen {
    fn filter_completion(&self, query: &str) -> Option<Completion> {
        if self.tasks.session.open() {
            let record = self.tasks.selected()?;
            return value(
                query.trim(),
                crate::task_center::search_fields(record).map(|field| field.into_owned()),
            )
            .map(Completion::Task);
        }
        match &self.mode {
            Mode::Browsing => selected(
                &self.rows(),
                self.repository_viewport.selected(),
                query,
                |row| [self.repository_search_field.text(&row.path).into_owned()],
            ),
            Mode::Management(session) => {
                selected(&session.rows(), session.state.selected(), query, |index| {
                    path_fields(&session.entries[*index].path)
                })
            }
            Mode::Files(files) => file_completion(files, query),
            Mode::Changes(session) => file_completion(&session.files, query),
            Mode::Browser(session) => value(
                query,
                [session
                    .selected_row()?
                    .entry
                    .path
                    .to_string_lossy()
                    .into_owned()],
            )
            .map(Completion::Browser),
            Mode::Branches(view) => {
                let branches::View::List(session) = view.as_ref() else {
                    return None;
                };
                selected(&session.rows(), session.state.selected(), query, |entry| {
                    [entry.name.clone()]
                })
            }
            Mode::Stashes(view) if let stashes::View::List(session) = view.as_ref() => {
                selected(&session.rows(), session.state.selected(), query, |entry| {
                    [entry.message.clone(), entry.oid.to_string()]
                })
            }
            Mode::History(_) | Mode::Stashes(_) => {
                let inspect = self.mode.inspection()?;
                selected(&inspect.rows(), inspect.files.selected(), query, |file| {
                    [file.location.name().to_string_lossy().into_owned()]
                })
            }
            _ => None,
        }
    }

    pub(crate) fn complete_filter(&mut self) -> bool {
        let InputBar::Search(editor) = self.active_bar() else {
            return false;
        };
        if self.active_search_support().mode() != Some(SearchMode::Filter)
            || editor.text.trim().is_empty()
            || editor.cursor != editor.text.len()
        {
            return false;
        }
        let Some(completion) = self.filter_completion(&editor.text.to_lowercase()) else {
            return false;
        };
        let query = match &completion {
            Completion::List { query, .. }
            | Completion::Browser(query)
            | Completion::Task(query) => query,
        };
        if *query == editor.text {
            return false;
        }
        *self.active_bar_mut() = InputBar::Search(Editor::new(query.clone()));
        self.message.clear();
        match completion {
            Completion::Browser(query) => {
                let Mode::Browser(session) = &mut self.mode else {
                    unreachable!("file browser completion requires the file browser");
                };
                session.complete_search(query);
                self.poll_browser();
            }
            Completion::Task(_) => self.normalize_bar(),
            Completion::List { index, .. } => {
                self.normalize_bar();
                match &mut self.mode {
                    Mode::Browsing => self.select(index),
                    Mode::Management(session) => session.state.select(Some(index)),
                    Mode::Files(files) => files.locate(index),
                    Mode::Changes(session) => session.files.locate(index),
                    Mode::Branches(view) => {
                        let branches::View::List(session) = view.as_mut() else {
                            unreachable!("branch completion requires branch rows");
                        };
                        session.state.select(Some(index));
                    }
                    Mode::Stashes(view) => match view.as_mut() {
                        stashes::View::List(session) => session.state.select(Some(index)),
                        stashes::View::Detail { inspect, .. } => inspect.files.select(Some(index)),
                        _ => unreachable!("stash completion requires list rows"),
                    },
                    Mode::History(_) => self
                        .mode
                        .inspection_mut()
                        .expect("history completion requires file rows")
                        .files
                        .select(Some(index)),
                    _ => unreachable!("list completion requires selectable rows"),
                }
            }
        }
        true
    }
}
