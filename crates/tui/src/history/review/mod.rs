//! A logical file anchor keeps a continuous review stable while files load or rows reflow.
mod layout;
mod load;
use crate::Service;
use engine::local::history::Detail;
pub(crate) use layout::{Layout, Rows};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, Default)]
struct Anchor {
    file: usize,
    row: usize,
}
#[derive(Debug, Clone)]
pub(crate) struct Review {
    bundle: Arc<Mutex<load::Bundle>>,
    pub layout: Layout,
    pub horizontal: crate::horizontal::Viewport,
    anchor: Anchor,
    followed: Option<usize>,
}
impl Review {
    pub fn new(detail: Arc<Detail>) -> Self {
        Self {
            bundle: Arc::new(Mutex::new(load::Bundle::new(detail))),
            layout: Layout::default(),
            horizontal: Default::default(),
            anchor: Anchor::default(),
            followed: None,
        }
    }
    pub fn poll(&self, service: &Arc<Service>) -> bool {
        self.bundle
            .lock()
            .expect("history review state")
            .poll(service)
    }
    #[cfg(test)]
    pub fn loaded(&self) -> bool {
        self.bundle.lock().expect("history review state").loaded()
    }
    pub fn symbol(&self) -> Option<&'static str> {
        self.bundle.lock().expect("history review state").symbol()
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
                self.anchor = Anchor { file, row: 0 };
            }
            self.followed = selected;
        }
        let entries = self
            .bundle
            .lock()
            .expect("history review state")
            .entries
            .clone();
        self.layout
            .arrange(&entries, files, area, options, paths, &mut self.anchor);
        self.horizontal.constrain(self.layout.horizontal_maximum());
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
