use engine::{
    cancel::Cancel,
    local::history::{self, Base, Continuation, Head, Label, Message, Parent, Request},
};
use git2::{Oid, Repository, Signature, Time};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
use test_support as support;

fn commit(repo: &Repository, number: usize, parents: &[Oid]) -> Oid {
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    let signature = Signature::new(
        "作者",
        "author@example.invalid",
        &Time::new(1_700_000_000 + number as i64, 480),
    )
    .unwrap();
    let parents: Vec<_> = parents
        .iter()
        .map(|oid| repo.find_commit(*oid).unwrap())
        .collect();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        &format!("提交 {number}"),
        &tree,
        &parents.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}

fn fingerprints(path: &Path) -> BTreeMap<String, Vec<u8>> {
    ["HEAD", "index", "config", "refs/heads/main"]
        .into_iter()
        .filter_map(|file| {
            let file_path = path.join(".git").join(file);
            file_path
                .is_file()
                .then(|| (file.to_owned(), fs::read(file_path).unwrap()))
        })
        .collect()
}

#[test]
fn unborn_and_detached_histories_do_not_require_an_attached_or_idle_worktree() {
    let temp = tempfile::tempdir().unwrap();
    support::init(temp.path());
    let path = fs::canonicalize(temp.path()).unwrap();
    let page = history::read(&path, Request::Start, &Cancel::default()).unwrap();
    assert_eq!(page.references.head, Head::Unborn("main".into()));
    assert!(page.entries.is_empty());
    assert!(matches!(page.continuation, Continuation::End));
    let repo = Repository::open(&path).unwrap();
    let oid = commit(&repo, 1, &[]);
    repo.set_head_detached(oid).unwrap();
    fs::write(path.join(".git/MERGE_HEAD"), format!("{oid}\n")).unwrap();
    let before = fingerprints(&path);
    let page = history::read(&path, Request::Start, &Cancel::default()).unwrap();
    assert_eq!(page.references.head, Head::Detached(oid));
    assert_eq!(page.entries.len(), 1);
    assert_eq!(fingerprints(&path), before);
}

#[test]
fn all_branch_and_tag_tips_form_one_topological_history_with_typed_labels() {
    let f = support::Fixture::create();
    f.commit("base.ts", "export const base = 1;\n");
    support::git(&f.repo, &["checkout", "-b", "feature"]);
    f.commit("feature.ts", "export const feature = 1;\n");
    support::git(&f.repo, &["checkout", "main"]);
    f.commit("main.ts", "export const main = 1;\n");
    support::git(
        &f.repo,
        &["merge", "--no-ff", "-m", "merge feature", "feature"],
    );
    support::git(&f.repo, &["tag", "-a", "v1", "-m", "release"]);
    support::git(&f.repo, &["tag", "tree-only", "HEAD^{tree}"]);
    support::git(
        &f.repo,
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
        ],
    );
    let before = fingerprints(&f.repo);
    let page = history::read(&f.repo, Request::Start, &Cancel::default()).unwrap();
    let positions: BTreeMap<_, _> = page
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.oid, index))
        .collect();
    assert_eq!(page.entries.len(), 5);
    assert_eq!(page.entries[0].parents().len(), 2);
    for entry in &page.entries {
        for parent in entry.parents() {
            assert!(positions[parent] > positions[&entry.oid]);
        }
    }
    let labels: Vec<_> = page.references.labels.values().flatten().collect();
    assert!(labels.contains(&&Label::Local("feature".into())));
    assert!(labels.contains(&&Label::Remote("origin/main".into())));
    assert!(labels.contains(&&Label::Tag("v1".into())));
    assert!(
        !labels
            .iter()
            .any(|label| label.name() == "tree-only" || label.name() == "origin/HEAD")
    );
    assert_eq!(fingerprints(&f.repo), before);
}

#[test]
fn pagination_keeps_frozen_tips_without_duplicates_when_branches_move() {
    let temp = tempfile::tempdir().unwrap();
    support::init(temp.path());
    let path = fs::canonicalize(temp.path()).unwrap();
    let repo = Repository::open(&path).unwrap();
    let mut expected = Vec::new();
    for number in 0..237 {
        expected.push(commit(
            &repo,
            number,
            &expected.last().copied().into_iter().collect::<Vec<_>>(),
        ));
    }
    let mut page = history::read(&path, Request::Start, &Cancel::default()).unwrap();
    assert_eq!(page.entries.len(), history::PAGE_SIZE);
    let frozen = page.references.clone();
    let new_tip = commit(&repo, 999, &[*expected.last().unwrap()]);
    let mut entries = Vec::new();
    loop {
        assert!(std::sync::Arc::ptr_eq(&frozen, &page.references));
        assert!(page.entries.len() <= history::PAGE_SIZE);
        entries.extend(page.entries.iter().map(|entry| entry.oid));
        match page.continuation {
            Continuation::End => break,
            Continuation::More(cursor) => {
                page = history::read(&path, Request::Continue(cursor), &Cancel::default()).unwrap()
            }
        }
    }
    expected.reverse();
    assert_eq!(entries, expected);
    assert_eq!(entries.iter().collect::<BTreeSet<_>>().len(), entries.len());
    assert!(!entries.contains(&new_tip));
    let fresh = history::read(&path, Request::Start, &Cancel::default()).unwrap();
    assert_eq!(fresh.entries[0].oid, new_tip);
}

#[test]
fn historical_diff_uses_blobs_and_literal_paths_without_touching_the_index_or_worktree() {
    let f = support::Fixture::create();
    let filename = "中文 [literal] file.ts";
    f.commit(filename, "export const value = \"old\";\n");
    f.commit(filename, "export const value = \"history\";\n");
    let repo = Repository::open(&f.repo).unwrap();
    let oid = repo.head().unwrap().target().unwrap();
    support::write(f.repo.join(filename), "export const value = \"index\";\n");
    support::git(&f.repo, &["add", "--", filename]);
    support::write(f.repo.join(filename), "export const value = \"working\";\n");
    let before = fingerprints(&f.repo);
    let detail = history::detail(&f.repo, oid, Parent::First, &Cancel::default()).unwrap();
    assert_eq!(detail.changes.len(), 1);
    let document = history::document(&detail, &detail.changes[0], &Cancel::default()).unwrap();
    let text = document.text();
    assert!(text.contains("-export const value = \"old\";"));
    assert!(text.contains("+export const value = \"history\";"));
    assert!(!text.contains("index\"") && !text.contains("working\""));
    let engine::local::files::patch::Document::Patch { files, .. } = document else {
        panic!()
    };
    assert!(
        matches!(&files[0].after,engine::local::files::patch::Source::Text {text,..} if text.contains("history"))
    );
    assert_eq!(fingerprints(&f.repo), before);
    assert_eq!(
        fs::read_to_string(f.repo.join(filename)).unwrap(),
        "export const value = \"working\";\n"
    );
}

#[test]
fn root_and_merge_comparisons_use_the_requested_tree() {
    let temp = tempfile::tempdir().unwrap();
    support::init(temp.path());
    let path = fs::canonicalize(temp.path()).unwrap();
    support::write(path.join("first.ts"), "export const first = 1;\n");
    support::git(&path, &["add", "."]);
    support::git(&path, &["commit", "-m", "root"]);
    let repo = Repository::open(&path).unwrap();
    let root = repo.head().unwrap().target().unwrap();
    let detail = history::detail(&path, root, Parent::First, &Cancel::default()).unwrap();
    assert_eq!(detail.base, Base::Empty);
    assert!(
        history::document(&detail, &detail.changes[0], &Cancel::default())
            .unwrap()
            .text()
            .contains("+export const first")
    );
    support::git(&path, &["checkout", "-b", "feature"]);
    support::write(path.join("feature.ts"), "export const feature = 1;\n");
    support::git(&path, &["add", "."]);
    support::git(&path, &["commit", "-m", "feature"]);
    support::git(&path, &["checkout", "main"]);
    support::write(path.join("main.ts"), "export const main = 1;\n");
    support::git(&path, &["add", "."]);
    support::git(&path, &["commit", "-m", "main"]);
    support::git(&path, &["merge", "--no-ff", "-m", "merge", "feature"]);
    let oid = repo.head().unwrap().target().unwrap();
    let first = history::detail(&path, oid, Parent::First, &Cancel::default()).unwrap();
    let second = history::detail(
        &path,
        oid,
        Parent::Commit(first.parents[1]),
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(first.changes[0].location.name(), Path::new("feature.ts"));
    assert_eq!(second.changes[0].location.name(), Path::new("main.ts"));
}

#[test]
fn long_unicode_messages_are_explicitly_bounded_and_cancellation_is_respected() {
    let temp = tempfile::tempdir().unwrap();
    support::init(temp.path());
    let path = fs::canonicalize(temp.path()).unwrap();
    let repo = Repository::open(&path).unwrap();
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    let signature = Signature::now("作者", "author@example.invalid").unwrap();
    let oid = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            &"中文".repeat(20_000),
            &tree,
            &[],
        )
        .unwrap();
    let detail = history::detail(&path, oid, Parent::First, &Cancel::default()).unwrap();
    assert!(matches!(detail.message, Message::Truncated(_)));
    assert!(detail.message.text().len() <= 64 * 1024);
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(
        history::read(&path, Request::Start, &cancel)
            .unwrap_err()
            .is::<engine::cancel::Cancelled>()
    );
    assert!(
        history::detail(&path, oid, Parent::First, &cancel)
            .unwrap_err()
            .is::<engine::cancel::Cancelled>()
    );
}

#[test]
fn shallow_boundaries_keep_real_parent_identity_and_never_compare_against_an_empty_tree() {
    let f = support::Fixture::create();
    f.commit("source.ts", "export const value = 1;\n");
    f.commit("source.ts", "export const value = 2;\n");
    let repo = Repository::open(&f.repo).unwrap();
    let tip = repo.head().unwrap().peel_to_commit().unwrap();
    let parent = tip.parent_id(0).unwrap();
    std::fs::write(f.repo.join(".git/shallow"), format!("{}\n", tip.id())).unwrap();
    let oid = parent.to_string();
    std::fs::remove_file(f.repo.join(".git/objects").join(&oid[..2]).join(&oid[2..])).unwrap();
    let page = history::read(&f.repo, Request::Start, &Cancel::default()).unwrap();
    assert_eq!(page.references.depth, history::Depth::Shallow);
    let entry = page
        .entries
        .iter()
        .find(|entry| entry.oid == tip.id())
        .unwrap();
    assert!(matches!(entry.ancestry, history::Ancestry::Boundary(_)));
    assert_eq!(entry.parents(), [parent]);
    assert!(entry.graph_parents().is_empty());
    let error = history::detail(&f.repo, tip.id(), Parent::First, &Cancel::default()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(i18n::tr!(HistoryParentUnavailable)),
        "{error:#}"
    );
}
