use engine::{
    cancel::Cancel,
    local::files::{
        self, Area,
        partial::{self, Action, LineId, Snapshot},
    },
};
use std::{collections::BTreeSet, fs, path::Path};
use test_support::*;

fn capture(f: &Fixture, name: &str, area: Area) -> Snapshot {
    let files = files::read(&f.repo).unwrap();
    let change = files
        .changes
        .iter()
        .find(|change| change.area == area && change.location.name() == Path::new(name))
        .unwrap();
    let document = files::document(&files, change, &Cancel::default()).unwrap();
    Snapshot::capture(&files, change, &document, &Cancel::default()).unwrap()
}
fn select(snapshot: &Snapshot, action: Action, ids: impl IntoIterator<Item = LineId>) {
    let plan = snapshot.prepare(action, ids.into_iter().collect()).unwrap();
    partial::execute(&plan, &Cancel::default()).unwrap();
}
fn old(line: u32) -> LineId {
    LineId::Old(line.try_into().unwrap())
}
fn new(line: u32) -> LineId {
    LineId::New(line.try_into().unwrap())
}

#[test]
fn selected_hunks_stage_and_unstage_without_writing_the_worktree() {
    let f = Fixture::create();
    let base = (1..=25)
        .map(|line| format!("line {line}\n"))
        .collect::<String>();
    f.commit("中文 空格.txt", &base);
    let changed = base
        .replace("line 2\n", "first\n")
        .replace("line 23\n", "last\n");
    write(f.repo.join("中文 空格.txt"), &changed);
    let snapshot = capture(&f, "中文 空格.txt", Area::Unstaged);
    assert_eq!(snapshot.hunks().len(), 2);
    select(&snapshot, Action::Stage, snapshot.hunks()[0].clone());
    assert_eq!(
        git(&f.repo, &["show", ":中文 空格.txt"]),
        base.replace("line 2\n", "first\n").trim_end()
    );
    assert_eq!(
        fs::read_to_string(f.repo.join("中文 空格.txt")).unwrap(),
        changed
    );
    let staged = capture(&f, "中文 空格.txt", Area::Staged);
    select(&staged, Action::Unstage, staged.hunks()[0].clone());
    assert_eq!(git(&f.repo, &["show", ":中文 空格.txt"]), base.trim_end());
    assert_eq!(
        fs::read_to_string(f.repo.join("中文 空格.txt")).unwrap(),
        changed
    );
}

#[test]
fn single_added_and_removed_lines_are_independent() {
    let f = Fixture::create();
    f.commit("file.txt", "before\ntail\n");
    write(f.repo.join("file.txt"), "after\ntail\n");
    select(
        &capture(&f, "file.txt", Area::Unstaged),
        Action::Stage,
        [new(1)],
    );
    assert_eq!(git(&f.repo, &["show", ":file.txt"]), "before\nafter\ntail");
    let staged = capture(&f, "file.txt", Area::Staged);
    select(
        &staged,
        Action::Unstage,
        staged.hunks().iter().flatten().copied(),
    );
    select(
        &capture(&f, "file.txt", Area::Unstaged),
        Action::Stage,
        [old(1)],
    );
    assert_eq!(git(&f.repo, &["show", ":file.txt"]), "tail");
    assert_eq!(
        fs::read_to_string(f.repo.join("file.txt")).unwrap(),
        "after\ntail\n"
    );
}

#[test]
fn discard_preserves_staged_content_and_unselected_worktree_changes() {
    let f = Fixture::create();
    f.commit("file.txt", "a\nb\nc\n");
    write(f.repo.join("file.txt"), "staged\nb\nc\n");
    git(&f.repo, &["add", "file.txt"]);
    write(f.repo.join("file.txt"), "work\nb\nlast\n");
    let index = fs::read(f.repo.join(".git/index")).unwrap();
    select(
        &capture(&f, "file.txt", Area::Unstaged),
        Action::Discard,
        [old(1), new(1)],
    );
    assert_eq!(
        fs::read_to_string(f.repo.join("file.txt")).unwrap(),
        "staged\nb\nlast\n"
    );
    assert_eq!(fs::read(f.repo.join(".git/index")).unwrap(), index);
}

#[test]
fn new_and_deleted_files_support_partial_content_and_complete_reversal() {
    let f = Fixture::create();
    write(f.repo.join("new.txt"), "one\ntwo");
    select(
        &capture(&f, "new.txt", Area::Untracked),
        Action::Stage,
        [new(1)],
    );
    assert_eq!(git(&f.repo, &["show", ":new.txt"]), "one");
    assert_eq!(
        fs::read_to_string(f.repo.join("new.txt")).unwrap(),
        "one\ntwo"
    );
    let staged = capture(&f, "new.txt", Area::Staged);
    select(
        &staged,
        Action::Unstage,
        staged.hunks().iter().flatten().copied(),
    );
    assert!(
        engine::local::open(&f.repo)
            .unwrap()
            .index()
            .unwrap()
            .get_path(Path::new("new.txt"), 0)
            .is_none()
    );
    select(
        &capture(&f, "new.txt", Area::Untracked),
        Action::Discard,
        [new(1)],
    );
    assert_eq!(fs::read_to_string(f.repo.join("new.txt")).unwrap(), "two");
    let untracked = capture(&f, "new.txt", Area::Untracked);
    select(
        &untracked,
        Action::Discard,
        untracked.hunks().iter().flatten().copied(),
    );
    assert!(!f.repo.join("new.txt").exists());
    f.commit("deleted.txt", "one\ntwo\n");
    fs::remove_file(f.repo.join("deleted.txt")).unwrap();
    select(
        &capture(&f, "deleted.txt", Area::Unstaged),
        Action::Discard,
        [old(2)],
    );
    assert_eq!(
        fs::read_to_string(f.repo.join("deleted.txt")).unwrap(),
        "two\n"
    );
}

#[test]
fn crlf_filters_and_missing_final_newlines_preserve_the_untouched_side() {
    let f = Fixture::create();
    f.commit("file.txt", "before\ntail");
    git(&f.repo, &["config", "core.autocrlf", "true"]);
    write(f.repo.join("file.txt"), "after\r\ntail");
    let snapshot = capture(&f, "file.txt", Area::Unstaged);
    select(&snapshot, Action::Stage, [old(1), new(1)]);
    assert_eq!(
        engine::local::open(&f.repo)
            .unwrap()
            .find_blob(
                engine::local::open(&f.repo)
                    .unwrap()
                    .index()
                    .unwrap()
                    .get_path(Path::new("file.txt"), 0)
                    .unwrap()
                    .id
            )
            .unwrap()
            .content(),
        b"after\ntail"
    );
    assert_eq!(fs::read(f.repo.join("file.txt")).unwrap(), b"after\r\ntail");
    write(f.repo.join("file.txt"), "again\r\ntail");
    select(
        &capture(&f, "file.txt", Area::Unstaged),
        Action::Discard,
        [old(1), new(1)],
    );
    assert_eq!(fs::read(f.repo.join("file.txt")).unwrap(), b"after\r\ntail");
}

#[test]
fn stale_content_index_and_external_locks_reject_without_overwriting() {
    let f = Fixture::create();
    f.commit("file.txt", "before\n");
    write(f.repo.join("file.txt"), "after\n");
    let snapshot = capture(&f, "file.txt", Area::Unstaged);
    let plan = snapshot
        .prepare(
            Action::Stage,
            snapshot.hunks().iter().flatten().copied().collect(),
        )
        .unwrap();
    write(f.repo.join("file.txt"), "external\n");
    assert!(partial::execute(&plan, &Cancel::default()).is_err());
    assert_eq!(git(&f.repo, &["show", ":file.txt"]), "before");
    let snapshot = capture(&f, "file.txt", Area::Unstaged);
    let plan = snapshot
        .prepare(Action::Discard, BTreeSet::from([old(1), new(1)]))
        .unwrap();
    write(f.repo.join(".git/index.lock"), "external lock");
    assert!(partial::execute(&plan, &Cancel::default()).is_err());
    assert_eq!(
        fs::read_to_string(f.repo.join("file.txt")).unwrap(),
        "external\n"
    );
    assert_eq!(
        fs::read_to_string(f.repo.join(".git/index.lock")).unwrap(),
        "external lock"
    );
    fs::remove_file(f.repo.join(".git/index.lock")).unwrap();
    git(&f.repo, &["add", "file.txt"]);
    assert!(partial::execute(&plan, &Cancel::default()).is_err());
    assert_eq!(
        fs::read_to_string(f.repo.join("file.txt")).unwrap(),
        "external\n"
    );
}
