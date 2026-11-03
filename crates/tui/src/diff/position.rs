//! Preserve source coordinates across context changes and refreshed index snapshots.
use super::*;
use engine::local::files::partial::LineId;

#[derive(Debug, Clone, Copy)]
pub(super) enum Coordinates {
    Both { old: NonZeroU32, new: NonZeroU32 },
    Old(NonZeroU32),
    New(NonZeroU32),
    Metadata,
}
impl Row {
    pub(super) fn coordinates(&self) -> Coordinates {
        match self {
            Self::Context { old, new } => Coordinates::Both {
                old: old.number,
                new: new.number,
            },
            Self::Removed(code) => Coordinates::Old(code.number),
            Self::Added(code) => Coordinates::New(code.number),
            Self::Information(_) | Self::Metadata(_) => Coordinates::Metadata,
        }
    }
}
impl Coordinates {
    fn preferred(self) -> Option<LineId> {
        match self {
            Self::Both { new, .. } | Self::New(new) => Some(LineId::New(new)),
            Self::Old(old) => Some(LineId::Old(old)),
            Self::Metadata => None,
        }
    }
    fn distance(self, line: LineId) -> Option<(bool, u32)> {
        let pair = match (self, line) {
            (Self::Both { old, .. } | Self::Old(old), LineId::Old(target)) => (old, target, false),
            (Self::Both { new, .. } | Self::New(new), LineId::New(target)) => (new, target, false),
            (Self::New(new), LineId::Old(target)) => (new, target, true),
            (Self::Old(old), LineId::New(target)) => (old, target, true),
            (Self::Metadata, _) => return None,
        };
        Some((pair.2, pair.0.get().abs_diff(pair.1.get())))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Position {
    line: LineId,
    continuation: usize,
    horizontal: usize,
}
impl Cache {
    pub(crate) fn source_position(&self, scroll: usize, horizontal: usize) -> Option<Position> {
        let anchor = match self.rows.get(scroll)?.anchors() {
            Anchors::One(anchor) | Anchors::Pair(_, anchor) => anchor,
        };
        let (row, line) = self
            .coordinates
            .iter()
            .enumerate()
            .skip(anchor.row)
            .find_map(|(row, source)| source.preferred().map(|line| (row, line)))
            .or_else(|| {
                self.coordinates
                    .iter()
                    .enumerate()
                    .take(anchor.row)
                    .rev()
                    .find_map(|(row, source)| source.preferred().map(|line| (row, line)))
            })?;
        Some(Position {
            line,
            continuation: if row == anchor.row {
                scroll.saturating_sub(self.locate(anchor))
            } else {
                0
            },
            horizontal,
        })
    }
    pub(crate) fn locate_position(&self, position: &Position) -> Option<usize> {
        let row = self
            .coordinates
            .iter()
            .enumerate()
            .filter_map(|(row, source)| {
                source
                    .distance(position.line)
                    .map(|distance| (row, distance))
            })
            .min_by_key(|(_, distance)| *distance)?
            .0;
        let start = self.locate(Anchor { row });
        let height = self.rows[start..]
            .iter()
            .take_while(|visual| visual.anchors().contains(Anchor { row }))
            .count();
        Some(start + position.continuation.min(height.saturating_sub(1)))
    }
}
impl View {
    pub(crate) fn position(&self, scroll: usize) -> Option<Position> {
        self.cache
            .as_ref()?
            .source_position(scroll, self.horizontal.offset())
    }
    pub(crate) fn restore_position(&mut self, position: Position) {
        self.reading = Some(position);
    }
    pub(super) fn restore_reading(&mut self, scroll: &mut usize) {
        if let Some(position) = self.reading.take() {
            if let Some(cache) = &self.cache
                && let Some(restored) = cache.locate_position(&position)
            {
                *scroll = restored;
            }
            self.horizontal.seek(position.horizontal);
            if let Interaction::Editable(Some(marking)) = &mut self.interaction {
                marking.reveal = false;
            }
        }
    }
}
