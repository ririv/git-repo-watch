//! Per-file opening choices retain independent viewport positions.
use super::*;
use engine::local::browser::Node;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextMode {
    Source,
    Preview,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Renderer {
    Markdown,
    Svg,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Opening {
    Standard,
    Text { renderer: Renderer, mode: TextMode },
}
impl Opening {
    pub fn in_mode(self, mode: TextMode) -> Self {
        match self {
            Self::Standard => Self::Standard,
            Self::Text { renderer, .. } => Self::Text { renderer, mode },
        }
    }
    pub(super) fn for_entry(entry: &Entry) -> Self {
        if !matches!(entry.node, Node::File(_)) {
            return Self::Standard;
        }
        let extension = entry.path.extension().and_then(|value| value.to_str());
        let renderer = match extension.map(str::to_ascii_lowercase).as_deref() {
            Some("md" | "markdown" | "mdown" | "mkd") => Renderer::Markdown,
            Some("svg") => Renderer::Svg,
            _ => return Self::Standard,
        };
        Self::Text {
            renderer,
            mode: TextMode::Source,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Standard
            | Self::Text {
                mode: TextMode::Source,
                ..
            } => i18n::tr!(FileSource),
            Self::Text {
                renderer: Renderer::Markdown,
                ..
            } => i18n::tr!(FileMarkdownPreview),
            Self::Text {
                renderer: Renderer::Svg,
                ..
            } => i18n::tr!(FileSvgPreview),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Presentation {
    Single(TextMode),
    Split(TextMode),
}
impl Presentation {
    pub fn active(self) -> TextMode {
        match self {
            Self::Single(mode) | Self::Split(mode) => mode,
        }
    }
    pub fn modes(self) -> &'static [TextMode] {
        match self {
            Self::Single(TextMode::Source) => &[TextMode::Source],
            Self::Single(TextMode::Preview) => &[TextMode::Preview],
            Self::Split(_) => &[TextMode::Source, TextMode::Preview],
        }
    }
    pub fn toggle(self) -> Self {
        match self {
            Self::Single(TextMode::Source) => Self::Single(TextMode::Preview),
            Self::Single(TextMode::Preview) | Self::Split(_) => Self::Single(TextMode::Source),
        }
    }
    pub fn split(self) -> Self {
        match self {
            Self::Single(_) => Self::Split(TextMode::Preview),
            Self::Split(mode) => Self::Single(mode),
        }
    }
}
#[derive(Debug, Clone)]
pub(crate) struct OpenTarget {
    pub path: PathBuf,
    pub renderer: Renderer,
    pub presentation: Presentation,
}
impl OpenTarget {
    pub fn opening(&self, mode: TextMode) -> Opening {
        Opening::Text {
            renderer: self.renderer,
            mode,
        }
    }
}
#[derive(Debug, Default, Clone)]
pub(crate) struct Position {
    pub scroll: usize,
    pub horizontal: horizontal::Viewport,
    pub find: crate::search::text::Draft,
}
#[derive(Debug)]
pub(super) struct Memory {
    pub(super) presentation: Presentation,
    pub(super) positions: BTreeMap<Opening, Position>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LayoutKey {
    Intrinsic,
    Markdown(u16),
}
impl Session {
    pub fn presentation(&self) -> Presentation {
        let initial = Presentation::Single(TextMode::Source);
        let Some(entry) = self.selected() else {
            return initial;
        };
        match Opening::for_entry(&entry) {
            Opening::Standard => initial,
            Opening::Text { .. } => self
                .memories
                .get(&entry.path)
                .map_or(initial, |memory| memory.presentation),
        }
    }
    pub fn open_target(&self) -> Option<OpenTarget> {
        let entry = self.selected()?;
        let Opening::Text { renderer, .. } = Opening::for_entry(&entry) else {
            return None;
        };
        Some(OpenTarget {
            path: entry.path,
            renderer,
            presentation: self.presentation(),
        })
    }
    pub(super) fn save_position(&mut self) {
        for mode in [TextMode::Source, TextMode::Preview] {
            let pane = self.panes.get(mode);
            let Some(key) = pane.preview.key().cloned() else {
                continue;
            };
            let position = pane.position.clone();
            let memory = self.memories.entry(key.path).or_insert_with(|| Memory {
                presentation: Presentation::Single(TextMode::Source),
                positions: BTreeMap::new(),
            });
            memory.positions.insert(key.opening, position);
        }
    }
    pub fn present(&mut self, target: &OpenTarget, presentation: Presentation) {
        if !self.open_target().is_some_and(|current| {
            current.path == target.path && current.renderer == target.renderer
        }) {
            return;
        }
        self.save_position();
        self.memories
            .entry(target.path.clone())
            .or_insert_with(|| Memory {
                presentation,
                positions: BTreeMap::new(),
            })
            .presentation = presentation;
        self.focus = Focus::Preview;
        self.sync_preview();
    }
    pub fn open_with(&mut self, target: &OpenTarget, mode: TextMode) {
        self.present(target, Presentation::Single(mode));
    }
    pub fn focus_mode(&mut self, mode: TextMode) {
        if let Some(target) = self.open_target()
            && matches!(target.presentation, Presentation::Split(_))
        {
            self.present(&target, Presentation::Split(mode));
        }
    }
    pub fn pane(&mut self) -> &mut panes::Pane {
        let mode = self.presentation().active();
        self.panes.get_mut(mode)
    }
    pub fn preview_width(&mut self, mode: TextMode, width: u16) {
        let pane = self.panes.get_mut(mode);
        if pane.width != width {
            pane.width = width;
            self.sync_preview();
        }
    }
    pub fn cycle_focus(&mut self, reverse: bool) {
        let modes = self.presentation().modes();
        let current = match self.focus {
            Focus::Tree => 0,
            Focus::Preview => {
                1 + modes
                    .iter()
                    .position(|mode| *mode == self.presentation().active())
                    .expect("visible preview")
            }
        };
        let next = if reverse {
            (current + modes.len()) % (modes.len() + 1)
        } else {
            (current + 1) % (modes.len() + 1)
        };
        if next == 0 {
            self.focus = Focus::Tree;
        } else {
            self.focus = Focus::Preview;
            self.focus_mode(modes[next - 1]);
        }
    }
}
