//! Independent bounded readers and viewports for source and rendered content.
use super::{opening::Position, *};
#[derive(Debug)]
pub(crate) struct Pane {
    pub preview: Preview,
    pub position: Position,
    pub layout: crate::flow::Cache,
    pub width: u16,
    pub find: crate::search::text::Index,
    reader: Worker<Key, preview::Document>,
    revision: u64,
}
impl Default for Pane {
    fn default() -> Self {
        Self {
            preview: Preview::Empty,
            position: Position::default(),
            layout: Default::default(),
            width: 80,
            find: Default::default(),
            reader: Worker::default(),
            revision: 0,
        }
    }
}
impl Pane {
    pub fn clear(&mut self) -> bool {
        let changed = !matches!(self.preview, Preview::Empty);
        self.preview = Preview::Empty;
        self.position = Position::default();
        self.layout = Default::default();
        self.find = Default::default();
        self.reader.clear();
        changed
    }
    pub fn sync(&mut self, path: &Path, opening: Opening, position: Position) -> bool {
        let layout = match opening {
            Opening::Text {
                renderer: Renderer::Markdown,
                mode: TextMode::Preview,
            } => LayoutKey::Markdown(self.width.max(1)),
            _ => LayoutKey::Intrinsic,
        };
        if self
            .preview
            .key()
            .is_some_and(|key| key.path == path && key.opening == opening && key.layout == layout)
        {
            return false;
        }
        self.position = position;
        self.layout = Default::default();
        self.find = Default::default();
        self.revision += 1;
        self.preview = Preview::Loading(Key {
            revision: self.revision,
            path: path.to_owned(),
            opening,
            layout,
        });
        true
    }
    pub fn search(&mut self, query: String) {
        if self.position.find.set_query(query) {
            self.reindex();
        }
    }
    pub fn reindex(&mut self) {
        if let Preview::Ready { document, .. } = &self.preview
            && let preview::Document::Text { text, gutter, .. } = document.as_ref()
        {
            self.find.rebuild(text, *gutter, &mut self.position.find);
        }
    }
    pub fn searchable(&self) -> bool {
        matches!(&self.preview, Preview::Ready { document, .. } if matches!(document.as_ref(), preview::Document::Text { .. }))
    }
    fn pending(&self) -> bool {
        self.reader.pending() || matches!(self.preview, Preview::Loading(_))
    }
    fn poll(&mut self, service: &Arc<Service>, root: &Path) -> bool {
        let mut changed = false;
        if let Preview::Loading(key) = &self.preview
            && self.reader.requested() != Some(key)
        {
            let service = service.clone();
            let root = root.to_owned();
            let path = key.path.clone();
            let opening = key.opening;
            let width = match key.layout {
                LayoutKey::Markdown(width) => width,
                LayoutKey::Intrinsic => 80,
            };
            self.reader.request(key.clone(), move |cancel| {
                preview::prepare(
                    service.browse_file(&root, &path, &cancel)?,
                    opening,
                    width,
                    &root,
                    &cancel,
                )
            });
            changed = true;
        }
        if let Some((key, result)) = self.reader.poll()
            && self.preview.key() == Some(&key)
        {
            self.preview = match result {
                Ok(document) => Preview::Ready {
                    key,
                    document: Arc::new(document),
                },
                Err(error) => Preview::Failed { key, error },
            };
            self.reindex();
            changed = true;
        }
        changed
    }
}
#[derive(Debug, Default)]
pub(crate) struct Panes {
    source: Pane,
    rendered: Pane,
    target: Option<Target>,
    pub feedback: crate::loading::Feedback<Snapshot>,
}

#[derive(Debug, Clone)]
pub(crate) struct Target {
    pub path: PathBuf,
    pub presentation: opening::Presentation,
    pub opening: Opening,
}
#[derive(Debug, Clone)]
pub(crate) struct View {
    pub preview: Preview,
    pub position: Position,
    pub layout: crate::flow::Cache,
    pub find: crate::search::text::Index,
}
#[derive(Debug)]
pub(crate) struct Snapshot {
    pub target: Target,
    source: View,
    rendered: View,
}
impl Snapshot {
    pub fn get_mut(&mut self, mode: TextMode) -> &mut View {
        match mode {
            TextMode::Source => &mut self.source,
            TextMode::Preview => &mut self.rendered,
        }
    }
}
impl Panes {
    pub fn invalidate(&mut self) {
        let previous = self.snapshot();
        for pane in [&mut self.source, &mut self.rendered] {
            if let Some(mut key) = pane.preview.key().cloned() {
                pane.reader.clear();
                pane.revision += 1;
                key.revision = pane.revision;
                pane.preview = Preview::Loading(key);
            }
        }
        self.feedback.begin(previous, Instant::now());
    }

    fn snapshot(&self) -> Option<Snapshot> {
        // 仍在静默等待时沿用原快照，避免快速连按将中间文件的半成品展示出来。
        if self.feedback.quiet() {
            return None;
        }
        let view = |pane: &Pane| View {
            preview: pane.preview.clone(),
            position: pane.position.clone(),
            layout: pane.layout.clone(),
            find: pane.find.clone(),
        };
        Some(Snapshot {
            target: self.target.clone()?,
            source: view(&self.source),
            rendered: view(&self.rendered),
        })
    }
    pub fn sync(
        &mut self,
        entry: Option<&Entry>,
        presentation: opening::Presentation,
        positions: [Position; 2],
        now: Instant,
    ) {
        let Some(entry) = entry else {
            self.clear();
            return;
        };
        let previous = self.snapshot();
        let opening = Opening::for_entry(entry);
        let mut changed = false;
        for (mode, position) in [TextMode::Source, TextMode::Preview]
            .into_iter()
            .zip(positions)
        {
            let pane = self.get_mut(mode);
            changed |= if presentation.modes().contains(&mode) {
                pane.sync(&entry.path, opening.in_mode(mode), position)
            } else {
                pane.clear()
            };
        }
        self.target = Some(Target {
            path: entry.path.clone(),
            presentation,
            opening,
        });
        if changed {
            self.feedback.begin(previous, now);
        }
    }
    pub fn get(&self, mode: TextMode) -> &Pane {
        match mode {
            TextMode::Source => &self.source,
            TextMode::Preview => &self.rendered,
        }
    }
    pub fn get_mut(&mut self, mode: TextMode) -> &mut Pane {
        match mode {
            TextMode::Source => &mut self.source,
            TextMode::Preview => &mut self.rendered,
        }
    }
    pub fn clear(&mut self) {
        self.source.clear();
        self.rendered.clear();
        self.target = None;
        self.feedback.finish();
    }
    pub fn pending(&self) -> bool {
        self.source.pending() || self.rendered.pending()
    }
    pub fn poll(&mut self, service: &Arc<Service>, root: &Path) -> bool {
        self.poll_at(service, root, Instant::now())
    }
    pub fn poll_at(&mut self, service: &Arc<Service>, root: &Path, now: Instant) -> bool {
        let mut changed = self.source.poll(service, root) | self.rendered.poll(service, root);
        let loading = self.target.as_ref().is_some_and(|target| {
            target
                .presentation
                .modes()
                .iter()
                .any(|mode| matches!(self.get(*mode).preview, Preview::Loading(_)))
        });
        if loading {
            changed |= self.feedback.tick(now);
        } else {
            changed |= self.feedback.finish();
        }
        changed
    }
}
