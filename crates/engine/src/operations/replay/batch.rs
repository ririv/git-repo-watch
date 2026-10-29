//! A reviewed non-empty sequence is passed to Git as one native cherry-pick operation.
use super::*;
use std::collections::BTreeSet;
pub const MAX_COMMITS: usize = 512;

#[derive(Debug, Clone)]
pub struct Batch {
    pub first: Start,
    pub second: Start,
    pub rest: Vec<Start>,
    pub(super) start: Oid,
}
impl Batch {
    pub fn steps(&self) -> impl DoubleEndedIterator<Item = &Start> {
        std::iter::once(&self.first)
            .chain(std::iter::once(&self.second))
            .chain(&self.rest)
    }
}

pub fn prepare(path: &Path, commits: &[Oid], cancel: &Cancel) -> Result<Plan> {
    ensure!(commits.len() >= 2, i18n::tr!(ReplayBatchAtLeastTwo));
    ensure!(commits.len() <= MAX_COMMITS, i18n::tr!(ReplayBatchTooLarge));
    let mut seen = BTreeSet::new();
    let mut steps = Vec::with_capacity(commits.len());
    let repo = local::open(path)?;
    local::guard::clean_tracked(&repo)?;
    let guard = Guard::capture(path, cancel)?;
    for oid in commits {
        cancel.check()?;
        ensure!(seen.insert(*oid), i18n::tr!(ReplayDuplicateCommit));
        let candidate = super::candidate(&repo, path, Kind::CherryPick, *oid, guard.clone())?;
        ensure!(candidate.parents.len() <= 1, i18n::tr!(ReplayBatchMerge));
        steps.push(super::describe(&repo, candidate, Mainline::Ordinary)?);
    }
    let mut steps = steps.into_iter();
    let first = steps.next().expect("non-empty selection was checked");
    let second = steps.next().expect("batch requires at least two commits");
    // 整批共用同一份仓库状态，只在准备前后核验，不按提交重复扫描索引。
    first.candidate.guard.validate(path, cancel)?;
    let start = first.candidate.guard.position.oid()?;
    Ok(Plan::Batch(Box::new(Batch {
        first,
        second,
        rest: steps.collect(),
        start,
    })))
}

pub(super) fn execute(git: &Git, batch: &Batch, cancel: &Cancel) -> Result<()> {
    let candidate = &batch.first.candidate;
    candidate.guard.validate(&candidate.path, cancel)?;
    let commits = batch
        .steps()
        .map(|step| step.candidate.commit)
        .collect::<Vec<_>>();
    git.cherry_pick_sequence(&candidate.path, &commits, cancel)
}
