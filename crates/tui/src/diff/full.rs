//! Expand unchanged source intervals around an immutable Git patch.
use super::{Cancel, Extent, Metadata, PatchLine};
use anyhow::{Result, ensure};
use engine::local::files::patch::{Source, SourceOmission};
use std::num::NonZeroU32;

pub(super) enum Expansion {
    Text(Vec<PatchLine>),
    NonText,
    Unavailable(&'static str),
}

pub(super) fn expand(
    before: &Source,
    after: &Source,
    patch: &[PatchLine],
    extent: Extent,
    cancel: &Cancel,
) -> Result<Expansion> {
    if [before, after]
        .iter()
        .any(|source| matches!(source, Source::Unavailable(SourceOmission::TooLarge)))
    {
        return Ok(Expansion::Unavailable(i18n::tr!(DiffFullUnavailable)));
    }
    let (Some(old), Some(new)) = (text(before), text(after)) else {
        return Ok(Expansion::NonText);
    };
    if extent == Extent::Truncated {
        return Ok(Expansion::Unavailable(i18n::tr!(DiffFullTruncated)));
    }
    let old = old.split_inclusive('\n').collect::<Vec<_>>();
    let new = new.split_inclusive('\n').collect::<Vec<_>>();
    if old.len().max(new.len()) > 50_000 {
        return Ok(Expansion::Unavailable(i18n::tr!(DiffFullLineLimit)));
    }
    let mut expanded = Vec::new();
    let mut cursor = Cursor { old: 0, new: 0 };
    for line in patch {
        cancel.check()?;
        let gap = match line {
            PatchLine::Context { old, new, .. } => {
                let old_gap = (old.get() as usize - 1).checked_sub(cursor.old);
                let new_gap = (new.get() as usize - 1).checked_sub(cursor.new);
                ensure!(
                    old_gap.is_some() && old_gap == new_gap,
                    i18n::tr!(InvalidDiffStructure)
                );
                old_gap.unwrap()
            }
            PatchLine::Added { new, .. } => (new.get() as usize - 1)
                .checked_sub(cursor.new)
                .ok_or_else(|| anyhow::anyhow!(i18n::tr!(InvalidDiffStructure)))?,
            PatchLine::Removed { old, .. } => (old.get() as usize - 1)
                .checked_sub(cursor.old)
                .ok_or_else(|| anyhow::anyhow!(i18n::tr!(InvalidDiffStructure)))?,
            PatchLine::Metadata {
                kind: Metadata::Notice,
                ..
            } => {
                expanded.push(line.clone());
                continue;
            }
            PatchLine::Metadata { .. } => continue,
        };
        cursor.context(gap, &old, &new, &mut expanded, cancel)?;
        match line {
            PatchLine::Context { .. } => {
                cursor.old += 1;
                cursor.new += 1;
            }
            PatchLine::Added { .. } => cursor.new += 1,
            PatchLine::Removed { .. } => cursor.old += 1,
            PatchLine::Metadata { .. } => unreachable!(),
        }
        expanded.push(line.clone());
    }
    ensure!(
        cursor.old <= old.len() && cursor.new <= new.len(),
        i18n::tr!(InvalidDiffStructure)
    );
    ensure!(
        old.len() - cursor.old == new.len() - cursor.new,
        i18n::tr!(DiffSourceChanged)
    );
    cursor.context(old.len() - cursor.old, &old, &new, &mut expanded, cancel)?;
    Ok(Expansion::Text(expanded))
}

struct Cursor {
    old: usize,
    new: usize,
}
impl Cursor {
    fn context(
        &mut self,
        count: usize,
        old: &[&str],
        new: &[&str],
        rows: &mut Vec<PatchLine>,
        cancel: &Cancel,
    ) -> Result<()> {
        for _ in 0..count {
            cancel.check()?;
            let (Some(before), Some(after)) = (old.get(self.old), new.get(self.new)) else {
                anyhow::bail!(i18n::tr!(InvalidDiffStructure));
            };
            ensure!(before == after, i18n::tr!(DiffSourceChanged));
            rows.push(PatchLine::Context {
                old: NonZeroU32::new(self.old as u32 + 1).unwrap(),
                new: NonZeroU32::new(self.new as u32 + 1).unwrap(),
                text: (*after).to_owned(),
            });
            self.old += 1;
            self.new += 1;
        }
        Ok(())
    }
}

fn text(source: &Source) -> Option<&str> {
    match source {
        Source::Text { text, .. } => Some(text),
        Source::Absent => Some(""),
        _ => None,
    }
}
