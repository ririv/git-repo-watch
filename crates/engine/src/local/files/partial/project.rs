//! Apply selected edits using stable old/new coordinates and each destination's exact line endings.
use super::*;
use anyhow::ensure;
pub(super) enum Direction {
    Forward,
    Reverse,
}
pub(super) fn apply(
    snapshot: &Snapshot,
    base: &Version,
    destination: &Version,
    selected: &BTreeSet<LineId>,
    direction: Direction,
) -> Result<Version> {
    let source = base.text().split_inclusive('\n').collect::<Vec<_>>();
    let target = destination.text().split_inclusive('\n').collect::<Vec<_>>();
    let mut cursor = 0usize;
    let mut delta = 0isize;
    let mut output = Vec::new();
    for block in &snapshot.blocks {
        let order = match direction {
            Direction::Forward => block.clone(),
            Direction::Reverse => block
                .iter()
                .filter(|id| matches!(id, LineId::New(_)))
                .chain(block.iter().filter(|id| matches!(id, LineId::Old(_))))
                .copied()
                .collect(),
        };
        for id in order {
            let (remove, number) = match (&direction, id) {
                (Direction::Forward, LineId::Old(number))
                | (Direction::Reverse, LineId::New(number)) => (true, number.get() as usize),
                (Direction::Forward, LineId::New(number))
                | (Direction::Reverse, LineId::Old(number)) => (false, number.get() as usize),
            };
            let at = if remove {
                number - 1
            } else {
                (number as isize - 1 - delta)
                    .try_into()
                    .map_err(|_| anyhow::anyhow!(i18n::tr!(PartialChanged)))?
            };
            ensure!(
                at >= cursor && at <= source.len(),
                i18n::tr!(PartialChanged)
            );
            output.extend(source[cursor..at].iter().copied());
            cursor = at;
            if remove {
                ensure!(at < source.len(), i18n::tr!(PartialChanged));
                if !selected.contains(&id) {
                    output.push(source[at]);
                }
                cursor += 1;
                delta -= 1;
            } else {
                ensure!(number <= target.len(), i18n::tr!(PartialChanged));
                if selected.contains(&id) {
                    output.push(target[number - 1]);
                }
                delta += 1;
            }
        }
    }
    output.extend(source[cursor..].iter().copied());
    ensure!(
        output.windows(2).all(|lines| lines[0].ends_with('\n')),
        i18n::tr!(PartialSelectionConflict)
    );
    let all = snapshot
        .hunks
        .iter()
        .flatten()
        .all(|id| selected.contains(id));
    if matches!(destination, Version::Absent) && all {
        return Ok(Version::Absent);
    }
    Ok(Version::Text {
        data: output.concat().into(),
        mode: match (base, destination) {
            (Version::Text { mode, .. }, _) | (Version::Absent, Version::Text { mode, .. }) => {
                *mode
            }
            (Version::Absent, Version::Absent) => anyhow::bail!(i18n::tr!(PartialChanged)),
        },
    })
}
