//! Align both sides to base line coordinates without deriving data from conflict markers.
use super::Version;
use crate::cancel::Cancel;
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Context(usize),
    Changed(usize),
}
impl Cell {
    pub fn line(self) -> Option<usize> {
        match self {
            Self::Empty => None,
            Self::Context(n) | Self::Changed(n) => Some(n),
        }
    }
}
#[derive(Debug)]
pub struct Row {
    pub base: Cell,
    pub current: Cell,
    pub incoming: Cell,
}
#[derive(Debug)]
pub enum Comparison {
    Text(Vec<Row>),
    NonText,
    LineLimit,
}
struct Projection {
    lines: Vec<Cell>,
    inserted: BTreeMap<usize, Vec<Cell>>,
}

pub(super) fn prepare(
    base: &Version,
    current: &Version,
    incoming: &Version,
    cancel: &Cancel,
) -> Result<Comparison> {
    let (Some(base), Some(current), Some(incoming)) =
        (base.text(), current.text(), incoming.text())
    else {
        return Ok(Comparison::NonText);
    };
    let count = base.lines().count();
    if [count, current.lines().count(), incoming.lines().count()]
        .into_iter()
        .any(|n| n > 50_000)
    {
        return Ok(Comparison::LineLimit);
    }
    let current = project(base, current, cancel)?;
    let incoming = project(base, incoming, cancel)?;
    combine(count, &current, &incoming, cancel)
}
pub(super) fn between(
    before: &super::super::patch::Source,
    after: &super::super::patch::Source,
    cancel: &Cancel,
) -> Result<Comparison> {
    use super::super::patch::Source;
    fn text(source: &Source) -> Option<&str> {
        match source {
            Source::Absent => Some(""),
            Source::Text { text, .. } => Some(text.as_str()),
            _ => None,
        }
    }
    let (Some(base), Some(side)) = (text(before), text(after)) else {
        return Ok(Comparison::NonText);
    };
    if base.lines().count() > 50_000 || side.lines().count() > 50_000 {
        return Ok(Comparison::LineLimit);
    }
    let projection = project(base, side, cancel)?;
    combine(base.lines().count(), &projection, &projection, cancel)
}
fn combine(
    count: usize,
    current: &Projection,
    incoming: &Projection,
    cancel: &Cancel,
) -> Result<Comparison> {
    let mut rows = Vec::new();
    for n in 0..=count {
        cancel.check()?;
        let ours = current.inserted.get(&n).map(Vec::as_slice).unwrap_or(&[]);
        let theirs = incoming.inserted.get(&n).map(Vec::as_slice).unwrap_or(&[]);
        for at in 0..ours.len().max(theirs.len()) {
            rows.push(Row {
                base: Cell::Empty,
                current: ours.get(at).copied().unwrap_or(Cell::Empty),
                incoming: theirs.get(at).copied().unwrap_or(Cell::Empty),
            });
        }
        if n < count {
            let changed = !matches!(current.lines[n], Cell::Context(_))
                || !matches!(incoming.lines[n], Cell::Context(_));
            rows.push(Row {
                base: if changed {
                    Cell::Changed(n)
                } else {
                    Cell::Context(n)
                },
                current: current.lines[n],
                incoming: incoming.lines[n],
            });
        }
    }
    Ok(Comparison::Text(rows))
}
fn project(base: &str, side: &str, cancel: &Cancel) -> Result<Projection> {
    let old = base.split_inclusive('\n').collect::<Vec<_>>();
    let new = side.split_inclusive('\n').collect::<Vec<_>>();
    let mut options = git2::DiffOptions::new();
    options.context_lines(0).interhunk_lines(0);
    let patch = git2::Patch::from_buffers(
        base.as_bytes(),
        None,
        side.as_bytes(),
        None,
        Some(&mut options),
    )?;
    let mut result = Projection {
        lines: vec![Cell::Empty; old.len()],
        inserted: BTreeMap::new(),
    };
    let (mut before, mut after) = (0, 0);
    for index in 0..patch.num_hunks() {
        cancel.check()?;
        let (hunk, _) = patch.hunk(index)?;
        let (removed, added) = (hunk.old_lines() as usize, hunk.new_lines() as usize);
        let start = if removed == 0 {
            hunk.old_start() as usize
        } else {
            hunk.old_start() as usize - 1
        };
        let target = if added == 0 {
            hunk.new_start() as usize
        } else {
            hunk.new_start() as usize - 1
        };
        ensure!(
            start >= before && target >= after && start - before == target - after,
            i18n::tr!(InvalidDiffStructure)
        );
        while before < start {
            ensure!(
                old.get(before) == new.get(after),
                i18n::tr!(InvalidDiffStructure)
            );
            result.lines[before] = Cell::Context(after);
            before += 1;
            after += 1;
        }
        ensure!(
            start + removed <= old.len() && target + added <= new.len(),
            i18n::tr!(InvalidDiffStructure)
        );
        for offset in 0..removed.min(added) {
            result.lines[start + offset] = Cell::Changed(target + offset);
        }
        if added > removed {
            result.inserted.insert(
                start + removed,
                (removed..added)
                    .map(|offset| Cell::Changed(target + offset))
                    .collect(),
            );
        }
        before = start + removed;
        after = target + added;
    }
    while before < old.len() {
        cancel.check()?;
        ensure!(
            old.get(before) == new.get(after),
            i18n::tr!(InvalidDiffStructure)
        );
        result.lines[before] = Cell::Context(after);
        before += 1;
        after += 1;
    }
    ensure!(after == new.len(), i18n::tr!(InvalidDiffStructure));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn text(text: &str) -> Version {
        Version::Present {
            path: "source.ts".into(),
            oid: git2::Oid::ZERO_SHA1,
            mode: 0o100644,
            source: crate::local::files::patch::Source::Text {
                path: "source.ts".into(),
                text: text.into(),
            },
        }
    }
    fn rows(base: &str, current: &str, incoming: &str) -> Vec<Row> {
        let Comparison::Text(rows) = prepare(
            &text(base),
            &text(current),
            &text(incoming),
            &Cancel::default(),
        )
        .unwrap() else {
            panic!("text")
        };
        rows
    }
    #[test]
    fn insertions_and_deletions_keep_common_tail_lines_aligned() {
        let rows = rows("a\nb\nc\n", "a\ninsert\nb\nc\n", "a\nc\n");
        assert_eq!(
            (rows[0].base, rows[0].current, rows[0].incoming),
            (Cell::Context(0), Cell::Context(0), Cell::Context(0))
        );
        assert_eq!(
            (rows[1].base, rows[1].current, rows[1].incoming),
            (Cell::Empty, Cell::Changed(1), Cell::Empty)
        );
        assert_eq!(
            (rows[2].base, rows[2].current, rows[2].incoming),
            (Cell::Changed(1), Cell::Context(2), Cell::Empty)
        );
        assert_eq!(
            (rows[3].base, rows[3].current, rows[3].incoming),
            (Cell::Context(2), Cell::Context(3), Cell::Context(1))
        );
    }
    #[test]
    fn empty_sources_independent_insertions_and_missing_final_newline_keep_coordinates() {
        let inserted = rows("", "one\ntwo", "other\n");
        assert_eq!(inserted.len(), 2);
        assert_eq!(
            (inserted[1].base, inserted[1].current, inserted[1].incoming),
            (Cell::Empty, Cell::Changed(1), Cell::Empty)
        );
        let ending = rows("line\n", "line", "line\n");
        assert_eq!(
            (ending[0].base, ending[0].current, ending[0].incoming),
            (Cell::Changed(0), Cell::Changed(0), Cell::Context(0))
        );
    }
    #[test]
    fn oversized_line_counts_report_the_comparison_limit() {
        assert!(matches!(
            prepare(
                &text("a\n".repeat(50_001).as_str()),
                &text("b"),
                &text("c"),
                &Cancel::default()
            )
            .unwrap(),
            Comparison::LineLimit
        ));
    }
}
