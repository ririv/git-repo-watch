//! Each selectable pair retains source coordinates, never an independent page.
use engine::{
    cancel::Cancel,
    local::files::{
        conflict::{Comparison, Snapshot, pair},
        patch::Document,
    },
};
pub(crate) use pair::Pair;
#[derive(Debug)]
pub(crate) struct Pairs([Comparison; Pair::ALL.len()]);
impl Pairs {
    pub fn prepare(
        snapshot: &Snapshot,
        result: &Document,
        cancel: &Cancel,
    ) -> anyhow::Result<Self> {
        let working = match result {
            Document::Patch { files, .. } if files.len() == 1 => Some(&files[0].after),
            Document::Notice(_) => None,
            _ => anyhow::bail!(i18n::tr!(InvalidDiffStructure)),
        };
        let sources = [
            Some(snapshot.base.source()),
            Some(snapshot.current.source()),
            Some(snapshot.incoming.source()),
            working,
        ];
        let compare = |selected: Pair| {
            let [left, right] = *selected.indices();
            match (sources[left], sources[right]) {
                (Some(left), Some(right)) => pair::compare(left, right, cancel),
                _ => Ok(Comparison::NonText),
            }
        };
        Ok(Self([
            compare(Pair::BaseCurrent)?,
            compare(Pair::BaseIncoming)?,
            compare(Pair::BaseResult)?,
            compare(Pair::CurrentIncoming)?,
            compare(Pair::CurrentResult)?,
            compare(Pair::ResultIncoming)?,
        ]))
    }
    pub fn get(&self, selected: Pair) -> &Comparison {
        &self.0[selected.index()]
    }
    pub fn reason(&self, selected: Pair) -> Option<&'static str> {
        match self.get(selected) {
            Comparison::Text(_) => None,
            Comparison::LineLimit => Some(i18n::tr!(ConflictLineLimit)),
            Comparison::NonText => Some(i18n::tr!(ConflictNonText)),
        }
    }
}
