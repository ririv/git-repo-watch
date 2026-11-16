//! Each selectable pair retains source coordinates, never an independent page.
use engine::{
    cancel::Cancel,
    local::files::{
        conflict::{Comparison, Snapshot, pair},
        patch::Source,
    },
};
pub(crate) use pair::Pair;
#[derive(Debug)]
pub(crate) struct Pairs([Comparison; Pair::ALL.len()]);
impl Pairs {
    pub fn prepare(snapshot: &Snapshot, working: &Source, cancel: &Cancel) -> anyhow::Result<Self> {
        let sources = [
            snapshot.base.source(),
            snapshot.current.source(),
            snapshot.incoming.source(),
            working,
        ];
        let compare = |selected: Pair| {
            let [left, right] = *selected.indices();
            pair::compare(sources[left], sources[right], cancel)
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
