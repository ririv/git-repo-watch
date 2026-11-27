//! A logical file anchor keeps a continuous review stable while files load or rows reflow.
mod layout;
mod load;
use super::Source;
use crate::Service;
pub(crate) use layout::{Layout, Rows, TitleRows};
use std::sync::{Arc, Mutex};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, Default)]
struct Anchor {
    file: usize,
    row: AnchorRow,
}
#[derive(Debug, Clone, Copy)]
enum AnchorRow {
    Title(usize),
    Body(usize),
    Divider,
}

impl Default for AnchorRow {
    fn default() -> Self {
        Self::Title(0)
    }
}

pub(crate) const TITLE_STATUS_WIDTH: usize = 2;

#[derive(Debug)]
pub(crate) struct SectionTitle {
    pub text: String,
    pub width: usize,
}

#[derive(Debug)]
struct Shared {
    bundle: Mutex<load::Bundle>,
    titles: Box<[SectionTitle]>,
}

#[derive(Debug, Clone)]
pub(crate) struct Review {
    shared: Arc<Shared>,
    pub layout: Layout,
    pub horizontal: crate::horizontal::Viewport,
    anchor: Anchor,
    followed: Option<usize>,
}
impl Review {
    pub fn new(detail: Source) -> Self {
        let titles = detail
            .changes()
            .into_iter()
            .map(|change| {
                let text = crate::view::safe(&change.location.display());
                SectionTitle {
                    width: text.width(),
                    text,
                }
            })
            .collect();
        Self {
            shared: Arc::new(Shared {
                bundle: Mutex::new(load::Bundle::new(detail)),
                titles,
            }),
            layout: Layout::default(),
            horizontal: Default::default(),
            anchor: Anchor::default(),
            followed: None,
        }
    }
    pub fn title(&self, file: usize) -> &SectionTitle {
        &self.shared.titles[file]
    }

    pub fn poll(&self, service: &Arc<Service>, theme: highlight::Theme) -> bool {
        self.shared
            .bundle
            .lock()
            .expect("history review state")
            .poll(service, theme)
    }
    #[cfg(test)]
    pub fn loaded(&self) -> bool {
        self.shared
            .bundle
            .lock()
            .expect("history review state")
            .loaded()
    }
    pub fn symbol(&self) -> Option<&'static str> {
        self.shared
            .bundle
            .lock()
            .expect("history review state")
            .symbol()
    }
    pub fn arrange(
        &mut self,
        files: &[usize],
        selected: Option<usize>,
        area: ratatui::layout::Rect,
        options: crate::diff::Options,
        paths: &[std::path::PathBuf],
    ) {
        if self.followed != selected {
            if let Some(file) = selected {
                self.anchor = Anchor {
                    file,
                    row: AnchorRow::default(),
                };
            }
            self.followed = selected;
        }
        let entries = self
            .shared
            .bundle
            .lock()
            .expect("history review state")
            .entries
            .clone();
        self.layout.arrange(
            layout::Content {
                entries: &entries,
                titles: &self.shared.titles,
            },
            files,
            area,
            options,
            paths,
            &mut self.anchor,
        );
    }
    pub fn seek(&mut self, offset: usize) {
        self.layout.offset = offset.min(self.layout.maximum());
        self.anchor = self.layout.anchor(self.layout.offset);
    }
    pub fn advance(&mut self, delta: isize) {
        self.seek(self.layout.offset.saturating_add_signed(delta));
    }
    pub fn visible_file(&self, selected: Option<usize>) -> Option<usize> {
        if self.followed == selected && !self.layout.sections.is_empty() {
            Some(self.anchor.file)
        } else {
            selected
        }
    }
}
