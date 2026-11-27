//! Cache shared styled layouts for continuous historical file sections.
use super::{Anchor, AnchorRow, SectionTitle, TITLE_STATUS_WIDTH, load::Entry};
use crate::diff::{Cache, HeaderVisibility, Options, Preference};
use ratatui::{layout::Rect, text::Line};
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
pub(crate) enum TitleRows {
    Single,
    Wrapped(Arc<[Line<'static>]>),
}

impl TitleRows {
    fn len(&self) -> usize {
        match self {
            Self::Single => 1,
            Self::Wrapped(lines) => lines.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Section {
    pub file: usize,
    pub start: usize,
    pub rows: Rows,
    pub title: TitleRows,
    boundary: Boundary,
}
impl Section {
    pub fn body_start(&self) -> usize {
        self.start + self.title.len()
    }

    pub fn height(&self) -> usize {
        self.rows.len()
            + self.title.len()
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
    caches: Vec<FileCache>,
}
#[derive(Debug, Clone, Default)]
struct FileCache {
    body: Option<Cached>,
    title: Option<WrappedTitle>,
}

#[derive(Debug, Clone)]
struct WrappedTitle {
    width: usize,
    lines: Arc<[Line<'static>]>,
}

#[derive(Debug, Clone)]
struct Cached {
    header: HeaderVisibility,
    layout: Arc<Cache>,
}
pub(super) struct Content<'a> {
    pub entries: &'a [Entry],
    pub titles: &'a [SectionTitle],
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
                        .filter_map(|row| match row {
                            crate::diff::VisualRow::Full { fixed, kind, .. }
                            | crate::diff::VisualRow::Columns { fixed, kind, .. } => match kind {
                                crate::diff::RowKind::Content => Some(*fixed),
                                crate::diff::RowKind::Annotation => None,
                            },
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
        content: Content<'_>,
        files: &[usize],
        area: Rect,
        options: Options,
        paths: &[PathBuf],
        anchor: &mut Anchor,
    ) {
        let Content { entries, titles } = content;
        let wrap = options.wrap;
        let width = usize::from(area.width).saturating_sub(TITLE_STATUS_WIDTH);
        self.caches.resize_with(entries.len(), FileCache::default);
        self.sections.clear();
        self.height = 0;
        self.page = area.height.into();
        for (index, &file) in files.iter().enumerate() {
            let rows = match &entries[file] {
                Entry::Pending => Rows::Pending,
                Entry::Failed(error) => Rows::Failed(error.clone()),
                Entry::Ready(view) => {
                    let cache = &mut self.caches[file].body;
                    let header_changed = cache
                        .as_ref()
                        .is_some_and(|cache| cache.header != options.header);
                    if cache.as_ref().is_none_or(|cache| {
                        header_changed
                            || cache.layout.width != area.width
                            || cache.layout.wrap != wrap
                            || cache.layout.context != options.context
                            || !cache.layout.matches_paths(paths)
                    }) {
                        let body_anchor = match anchor.row {
                            AnchorRow::Body(row) if file == anchor.file => Some(row),
                            _ => None,
                        };
                        let source_position = cache
                            .as_ref()
                            .filter(|cache| cache.layout.context != options.context)
                            .and_then(|cache| cache.layout.source_position(body_anchor?, 0));
                        let mut source_anchor = body_anchor.and_then(|row| {
                            cache
                                .as_ref()?
                                .layout
                                .rows
                                .get(row)
                                .map(|row| row.anchors().preferred())
                        });
                        if header_changed && let Some(source) = &mut source_anchor {
                            source.row = match options.header {
                                HeaderVisibility::Expanded if source.row >= view.header_start() => {
                                    source.row + view.header_rows()
                                }
                                HeaderVisibility::Collapsed
                                    if source.row >= view.header_start() =>
                                {
                                    source
                                        .row
                                        .saturating_sub(view.header_rows())
                                        .max(view.header_start())
                                }
                                _ => source.row,
                            };
                        }
                        let next = Arc::new(view.layout(
                            area.width,
                            Preference::Unified,
                            wrap,
                            paths,
                            options.header,
                            options.context,
                        ));
                        if let Some(position) = source_position {
                            anchor.row =
                                AnchorRow::Body(next.locate_position(&position).unwrap_or(0));
                        } else if cache
                            .as_ref()
                            .is_some_and(|cache| cache.layout.context != options.context)
                        {
                            if file == anchor.file {
                                anchor.row = AnchorRow::default();
                            }
                        } else if let Some(source_anchor) = source_anchor {
                            anchor.row = AnchorRow::Body(next.locate(source_anchor));
                        }
                        *cache = Some(Cached {
                            header: options.header,
                            layout: next,
                        });
                    }
                    Rows::Ready(
                        cache
                            .as_ref()
                            .expect("prepared history diff layout")
                            .layout
                            .clone(),
                    )
                }
            };
            let title = &titles[file];
            let title = if wrap && width > 0 && title.width > width {
                let cache = &mut self.caches[file].title;
                let wrapped = match cache {
                    Some(cached) if cached.width == width => cached,
                    _ => cache.insert(WrappedTitle {
                        width,
                        lines: crate::links::Line::from(Line::raw(title.text.clone()))
                            .wrap(width, false)
                            .into_iter()
                            .map(|line| line.text)
                            .collect(),
                    }),
                };
                TitleRows::Wrapped(wrapped.lines.clone())
            } else {
                TitleRows::Single
            };
            let section = Section {
                file,
                start: self.height,
                rows,
                title,
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
            .map(|section| match anchor.row {
                AnchorRow::Title(row) => section.start + row.min(section.title.len() - 1),
                AnchorRow::Body(row) => section.body_start() + row.min(section.rows.len() - 1),
                AnchorRow::Divider => section
                    .separator()
                    .unwrap_or(section.start + section.height() - 1),
            })
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
                row: if offset < section.body_start() {
                    AnchorRow::Title(offset - section.start)
                } else if section.separator() == Some(offset) {
                    AnchorRow::Divider
                } else {
                    AnchorRow::Body(offset - section.body_start())
                },
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
            *crate::diff::prepare_inline_comparison(
                Document::Notice(
                    "a long historical line for wrapping and scrolling\n".repeat(lines),
                ),
                crate::diff::Comparison::History {
                    before: Base::Empty,
                    after: Oid::ZERO_SHA1,
                },
                &Cancel::default(),
            )
            .unwrap(),
        ))
    }

    #[test]
    fn pending_predecessors_and_resizing_retain_the_requested_file_anchor() {
        let mut layout = Layout::default();
        let mut entries = vec![Entry::Pending; 3];
        let mut anchor = Anchor {
            file: 2,
            row: AnchorRow::default(),
        };
        let titles = (0..3)
            .map(|_| SectionTitle {
                text: "file".into(),
                width: 4,
            })
            .collect::<Vec<_>>();
        let area = Rect::new(0, 0, 60, 20);
        layout.arrange(
            Content {
                entries: &entries,
                titles: &titles,
            },
            &[0, 1, 2],
            area,
            Options {
                layout: Preference::Unified,
                wrap: false,
                header: HeaderVisibility::Collapsed,
                context: engine::state::view::DiffContext::Changes,
            },
            &[],
            &mut anchor,
        );

        assert_eq!(anchor.file, 2);
        entries[0] = ready(50);
        layout.arrange(
            Content {
                entries: &entries,
                titles: &titles,
            },
            &[0, 1, 2],
            area,
            Options {
                layout: Preference::Unified,
                wrap: false,
                header: HeaderVisibility::Collapsed,
                context: engine::state::view::DiffContext::Changes,
            },
            &[],
            &mut anchor,
        );

        assert_eq!(anchor.file, 2);
        assert!(layout.offset > 0);
        entries[1] = ready(60);
        for width in [60, 30, 80] {
            layout.arrange(
                Content {
                    entries: &entries,
                    titles: &titles,
                },
                &[0, 1, 2],
                Rect { width, ..area },
                Options {
                    layout: Preference::Unified,
                    wrap: true,
                    header: HeaderVisibility::Collapsed,
                    context: engine::state::view::DiffContext::Changes,
                },
                &[],
                &mut anchor,
            );

            assert_eq!(anchor.file, 2);
            let target = &layout.sections[2];
            assert!((layout.offset..layout.offset + area.height as usize).contains(&target.start));
        }
    }
}
