//! Independent bounded readers and viewports for source and rendered content.
use super::{opening::Position, *};
#[derive(Debug)]
pub(crate) struct Pane {
    pub preview: Preview,
    pub position: Position,
    pub width: u16,
    reader: Worker<Key, preview::Document>,
    revision: u64,
}
impl Default for Pane {
    fn default() -> Self {
        Self {
            preview: Preview::Empty,
            position: Position::default(),
            width: 80,
            reader: Worker::default(),
            revision: 0,
        }
    }
}
impl Pane {
    pub fn clear(&mut self) {
        self.preview = Preview::Empty;
        self.position = Position::default();
        self.reader.clear();
    }
    pub fn sync(&mut self, path: &Path, opening: Opening, position: Position) {
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
            return;
        }
        self.position = position;
        self.revision += 1;
        self.preview = Preview::Loading(Key {
            revision: self.revision,
            path: path.to_owned(),
            opening,
            layout,
        });
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
                Ok(document) => Preview::Ready { key, document },
                Err(error) => Preview::Failed { key, error },
            };
            changed = true;
        }
        changed
    }
}
#[derive(Debug, Default)]
pub(crate) struct Panes {
    source: Pane,
    rendered: Pane,
}
impl Panes {
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
    }
    pub fn pending(&self) -> bool {
        self.source.pending() || self.rendered.pending()
    }
    pub fn poll(&mut self, service: &Arc<Service>, root: &Path) -> bool {
        self.source.poll(service, root) | self.rendered.poll(service, root)
    }
}
