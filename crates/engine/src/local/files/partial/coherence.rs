//! Verify displayed edits still describe the captured source versions.
use super::*;
use crate::local::files::patch::{FilePatch, Line};

fn normalized(line: &str) -> String {
    line.replace("\r\n", "\n")
}
impl Snapshot {
    pub(super) fn check_document(&self, patch: &FilePatch) -> Result<()> {
        let (before, after) = match &self.layer {
            Layer::Staged { before, after } => (before, after),
            Layer::Working {
                before, indexed, ..
            } => (before, indexed),
        };
        let old = before.text().split_inclusive('\n').collect::<Vec<_>>();
        let new = after.text().split_inclusive('\n').collect::<Vec<_>>();
        let matches = |source: &[&str], number: NonZeroU32, text: &str| {
            source
                .get(number.get() as usize - 1)
                .is_some_and(|line| normalized(line) == normalized(text))
        };
        for line in &patch.lines {
            ensure!(
                match line {
                    Line::Added { new: number, text } => matches(&new, *number, text),
                    Line::Removed { old: number, text } => matches(&old, *number, text),
                    Line::Context {
                        old: left,
                        new: right,
                        text,
                    } => matches(&old, *left, text) && matches(&new, *right, text),
                    Line::Metadata { .. } => true,
                },
                i18n::tr!(PartialChanged)
            );
        }
        let all = self.hunks.iter().flatten().copied().collect();
        let complete = project::apply(self, before, after, &all, project::Direction::Forward)?;
        ensure!(complete.text() == after.text(), i18n::tr!(PartialChanged));
        Ok(())
    }
}
