//! Deterministic topological pages; continuation keeps the initial reference snapshot.
use super::*;
use crate::{cancel::Cancel, local::open};
use anyhow::{Context, Result, ensure};
use git2::{ErrorCode, ObjectType, Repository, Sort};
use std::{collections::BTreeSet, path::Path};

fn boundaries(repo: &Repository) -> Result<BTreeSet<Oid>> {
    let bytes = match std::fs::read(repo.commondir().join("shallow")) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(error) => return Err(error.into()),
    };
    std::str::from_utf8(&bytes)?
        .lines()
        .map(|line| Oid::from_str_ext(line, repo.object_format()).map_err(Into::into))
        .collect()
}

fn references(repo: &Repository, path: &Path, cancel: &Cancel) -> Result<References> {
    let boundaries = boundaries(repo)?;
    let head = match repo.head() {
        Ok(reference) => {
            let oid = reference.peel_to_commit()?.id();
            if repo.head_detached()? {
                Head::Detached(oid)
            } else {
                Head::Attached {
                    branch: reference
                        .shorthand()
                        .context(i18n::tr!(BranchNameIsNotUtf8))?
                        .to_owned(),
                    oid,
                }
            }
        }
        Err(error) if error.code() == ErrorCode::UnbornBranch => {
            let reference = repo.find_reference("HEAD")?;
            Head::Unborn(
                reference
                    .symbolic_target()?
                    .context(i18n::tr!(HeadHasNoSymbolicBranch))?
                    .strip_prefix("refs/heads/")
                    .context(i18n::tr!(HeadIsNotAttachedToALocal))?
                    .to_owned(),
            )
        }
        Err(error) => return Err(error.into()),
    };
    let mut labels: BTreeMap<Oid, Vec<Label>> = BTreeMap::new();
    let mut tips = BTreeSet::new();
    if let Some(oid) = head.oid() {
        tips.insert(oid);
    }
    for reference in repo.references()? {
        cancel.check()?;
        let reference = reference?;
        let name = reference
            .name()
            .context(i18n::tr!(HistoryReferenceNotUtf8))?;
        let label = if let Some(name) = name.strip_prefix("refs/heads/") {
            Label::Local(name.to_owned())
        } else if let Some(name) = name.strip_prefix("refs/remotes/") {
            // origin/HEAD 只是默认分支别名，不创建重复的分支标签或遍历入口。
            if reference.symbolic_target_bytes().is_some() {
                continue;
            }
            Label::Remote(name.to_owned())
        } else if let Some(name) = name.strip_prefix("refs/tags/") {
            Label::Tag(name.to_owned())
        } else {
            continue;
        };
        let object = reference.peel(ObjectType::Any)?;
        if matches!(label, Label::Tag(_)) && object.kind() != Some(ObjectType::Commit) {
            continue;
        }
        ensure!(
            object.kind() == Some(ObjectType::Commit),
            i18n::tr!(HistoryReferenceNotCommit)
        );
        let oid = object.id();
        tips.insert(oid);
        labels.entry(oid).or_default().push(label);
    }
    for values in labels.values_mut() {
        values.sort_by(|a, b| a.name().cmp(b.name()));
    }
    Ok(References {
        path: path.to_owned(),
        head,
        labels,
        depth: if !boundaries.is_empty() {
            Depth::Shallow
        } else {
            Depth::Complete
        },
        tips: tips.into_iter().collect(),
        boundaries,
    })
}

pub fn read(path: &Path, request: Request, cancel: &Cancel) -> Result<Page> {
    cancel.check()?;
    let repo = open(path)?;
    let (references, offset) = match request {
        Request::Start => (Arc::new(references(&repo, path, cancel)?), 0),
        Request::Continue(cursor) => {
            ensure!(
                cursor.references.path == path,
                i18n::tr!(HistoryCursorRepositoryChanged)
            );
            (cursor.references, cursor.offset)
        }
    };
    ensure!(
        boundaries(&repo)? == references.boundaries,
        i18n::tr!(HistorySnapshotUnavailable)
    );
    let mut walk = repo.revwalk()?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    for oid in &references.tips {
        walk.push(*oid)?;
    }
    for _ in 0..offset {
        cancel.check()?;
        walk.next()
            .context(i18n::tr!(HistorySnapshotUnavailable))??;
    }
    let mut entries = Vec::new();
    for _ in 0..PAGE_SIZE {
        cancel.check()?;
        let Some(oid) = walk.next() else {
            break;
        };
        let commit = repo.find_commit(oid?)?;
        let message =
            std::str::from_utf8(commit.message_bytes()).context(i18n::tr!(HistoryTextNotUtf8))?;
        let subject = message
            .split('\n')
            .next()
            .expect("split always yields a part")
            .trim();
        let author = commit.author();
        entries.push(Entry {
            oid: commit.id(),
            ancestry: super::parents::ancestry(&repo, &commit)?,
            subject: subject.chars().take(200).collect(),
            author: std::str::from_utf8(author.name_bytes())
                .context(i18n::tr!(HistoryTextNotUtf8))?
                .chars()
                .take(200)
                .collect(),
            seconds: commit.time().seconds(),
        });
    }
    cancel.check()?;
    let continuation = match walk.next() {
        None => Continuation::End,
        Some(oid) => {
            oid?;
            Continuation::More(Cursor {
                references: references.clone(),
                offset: offset + entries.len(),
            })
        }
    };
    ensure!(
        boundaries(&repo)? == references.boundaries,
        i18n::tr!(HistorySnapshotUnavailable)
    );
    Ok(Page {
        references,
        entries,
        continuation,
    })
}
