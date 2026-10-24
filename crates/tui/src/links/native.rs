//! OSC 8 output is a final pass over actual cells, never embedded in Ratatui symbols.
use super::Map;
use crossterm::{
    cursor::{RestorePosition, SavePosition},
    queue,
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
};
use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
    io::{self, Write},
};
#[derive(Default)]
pub(crate) struct Native {
    previous: BTreeSet<(u16, u16)>,
}
impl Native {
    pub fn flush(
        &mut self,
        writer: &mut impl Write,
        buffer: &Buffer,
        links: &Map,
    ) -> io::Result<()> {
        let current = links.cells.keys().copied().collect::<BTreeSet<_>>();
        let mut starts = BTreeSet::new();
        for y in buffer.area.y..buffer.area.bottom() {
            let mut x = buffer.area.x;
            while x < buffer.area.right() {
                starts.insert((y, x));
                x += unicode_width::UnicodeWidthStr::width(buffer[(x, y)].symbol()).max(1) as u16;
            }
        }
        let positions = self
            .previous
            .union(&current)
            .copied()
            .filter(|point| starts.contains(point))
            .collect::<Vec<_>>();
        if !positions.is_empty() {
            queue!(writer, SavePosition)?;
            let mut start = 0;
            while start < positions.len() {
                let (y, x) = positions[start];
                let target = links.cells.get(&(y, x));
                let mut end = start + 1;
                while end < positions.len()
                    && positions[end].0 == y
                    && positions[end].1
                        == positions[end - 1].1
                            + unicode_width::UnicodeWidthStr::width(
                                buffer[(positions[end - 1].1, y)].symbol(),
                            )
                            .max(1) as u16
                    && links.cells.get(&positions[end]) == target
                {
                    end += 1;
                }
                if let Some(target) = target {
                    let uri = target.uri();
                    let mut hash = std::collections::hash_map::DefaultHasher::new();
                    uri.hash(&mut hash);
                    write!(writer, "\x1b]8;id=link-{:x};{uri}\x1b\\", hash.finish())?;
                } else {
                    writer.write_all(b"\x1b]8;;\x1b\\")?;
                }
                CrosstermBackend::new(&mut *writer).draw(
                    positions[start..end]
                        .iter()
                        .map(|(y, x)| (*x, *y, &buffer[(*x, *y)])),
                )?;
                writer.write_all(b"\x1b]8;;\x1b\\")?;
                start = end;
            }
            queue!(writer, RestorePosition)?;
            writer.flush()?;
        }
        self.previous = current;
        Ok(())
    }
}
