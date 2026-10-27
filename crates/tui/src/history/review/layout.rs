//! File sections share styled diff layouts without copying prepared source rows.
use super::{Anchor, load::Entry};
use crate::diff::{Cache, Preference};
use ratatui::layout::Rect;
use std::{path::PathBuf, sync::Arc};

#[derive(Debug, Clone)]
pub(crate) enum Rows {
    Pending,
    Failed(Arc<str>),
    Ready(Arc<Cache>),
}

impl Rows {
    pub fn len(&self) -> usize {
        match self {
            Self::Pending | Self::Failed(_) => 1,
            Self::Ready(cache) => cache.rows.len().max(1),
        }
    }
}
#[derive(Debug, Clone, Copy)]
enum Boundary {
    Divider,
    End,
}
#[derive(Debug, Clone)]
pub(crate) struct Section {
    pub file: usize,
    pub start: usize,
    pub rows: Rows,
    boundary: Boundary,
}
impl Section {
    pub fn height(&self) -> usize {
        self.rows.len()
            + 1
            + match self.boundary {
                Boundary::Divider => 1,
                Boundary::End => 0,
            }
    }
    pub fn separator(&self) -> Option<usize> {
        match self.boundary {
            Boundary::Divider => Some(self.start + self.height() - 1),
            Boundary::End => None,
        }
    }
}
#[derive(Debug, Default, Clone)]
pub(crate) struct Layout {
    pub sections: Vec<Section>,
    pub offset: usize,
    pub height: usize,
    page: usize,
    caches: Vec<Option<Arc<Cache>>>,
}
impl Layout {
    pub fn maximum(&self) -> usize {
        self.height.saturating_sub(self.page)
    }
    pub fn horizontal_maximum(&self) -> usize {
        self.sections
            .iter()
            .filter_map(|section| match &section.rows {
                Rows::Ready(cache) => Some(cache.horizontal_maximum()),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }
    pub fn gutter_width(&self) -> usize {
        self.sections
            .iter()
            .filter_map(|section| match &section.rows {
                Rows::Ready(cache) => Some(
                    cache
                        .rows
                        .iter()
                        .map(|row| match row {
                            crate::diff::VisualRow::Full { fixed, .. }
                            | crate::diff::VisualRow::Columns { fixed, .. } => *fixed,
                        })
                        .max()
                        .unwrap_or(0),
                ),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }
    pub(super) fn arrange(
        &mut self,
        entries: &[Entry],
        files: &[usize],
        area: Rect,
        wrap: bool,
        paths: &[PathBuf],
        anchor: &mut Anchor,
    ) {
        self.caches.resize_with(entries.len(), || None);
        self.sections.clear();
        self.height = 0;
        self.page = area.height.into();
        for (index, &file) in files.iter().enumerate() {
            let rows = match &entries[file] {
                Entry::Pending => Rows::Pending,
                Entry::Failed(error) => Rows::Failed(error.clone()),
                Entry::Ready(view) => {
                    let cache = &mut self.caches[file];
                    if cache.as_ref().is_none_or(|cache| {
                        cache.width != area.width
                            || cache.wrap != wrap
                            || !cache.matches_paths(paths)
                    }) {
                        let source_anchor = (file == anchor.file && anchor.row > 0)
                            .then(|| {
                                cache
                                    .as_ref()
                                    .and_then(|cache| cache.rows.get(anchor.row - 1))
                                    .map(|row| row.anchors().preferred())
                            })
                            .flatten();
                        let next =
                            Arc::new(view.layout(area.width, Preference::Unified, wrap, paths));
                        if let Some(source_anchor) = source_anchor {
                            anchor.row = next.locate(source_anchor) + 1;
                        }
                        *cache = Some(next);
                    }
                    Rows::Ready(
                        cache
                            .as_ref()
                            .expect("prepared history diff layout")
                            .clone(),
                    )
                }
            };
            let section = Section {
                file,
                start: self.height,
                rows,
                boundary: if index + 1 < files.len() {
                    Boundary::Divider
                } else {
                    Boundary::End
                },
            };
            self.height += section.height();
            self.sections.push(section);
        }
        self.offset = self
            .sections
            .iter()
            .find(|section| section.file == anchor.file)
            .map(|section| section.start + anchor.row.min(section.height().saturating_sub(1)))
            .unwrap_or(0)
            .min(self.maximum());
        // 布局裁剪不覆盖逻辑锚点；后续文件加载或窗口变高后仍定位原文件。
    }
    pub(super) fn anchor(&self, offset: usize) -> Anchor {
        self.sections
            .iter()
            .rev()
            .find(|section| section.start <= offset)
            .map(|section| Anchor {
                file: section.file,
                row: offset - section.start,
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::{
        cancel::Cancel,
        local::{
            files::patch::Document,
            history::{Base, Oid},
        },
    };

    fn ready(lines: usize) -> Entry {
        Entry::Ready(Arc::from(
            *crate::diff::prepare_inline_history(
                Document::Notice(
                    "a long historical line for wrapping and scrolling\n".repeat(lines),
                ),
                Base::Empty,
                Oid::ZERO_SHA1,
                &Cancel::default(),
            )
            .unwrap(),
        ))
    }

    #[test]
    fn pending_predecessors_and_resizing_retain_the_requested_file_anchor() {
        let mut layout = Layout::default();
        let mut entries = vec![Entry::Pending; 3];
        let mut anchor = Anchor { file: 2, row: 0 };
        let area = Rect::new(0, 0, 60, 20);
        layout.arrange(&entries, &[0, 1, 2], area, false, &[], &mut anchor);
        assert_eq!(anchor.file, 2);
        entries[0] = ready(50);
        layout.arrange(&entries, &[0, 1, 2], area, false, &[], &mut anchor);
        assert_eq!(anchor.file, 2);
        assert!(layout.offset > 0);
        entries[1] = ready(60);
        for width in [60, 30, 80] {
            layout.arrange(
                &entries,
                &[0, 1, 2],
                Rect { width, ..area },
                true,
                &[],
                &mut anchor,
            );
            assert_eq!(anchor.file, 2);
            let target = &layout.sections[2];
            assert!((layout.offset..layout.offset + area.height as usize).contains(&target.start));
        }
    }
}
