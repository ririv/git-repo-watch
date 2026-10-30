use engine::{
    cancel::Cancel,
    local::files::{
        self, Area,
        partial::{self, Action, LineId, Snapshot},
    },
};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};
use test_support::*;

fn snapshot(path: &Path, name: &str, area: Area) -> Snapshot {
    let files = files::read(path).unwrap();
    let change = files
        .changes
        .iter()
        .find(|change| change.area == area && change.location.name() == Path::new(name))
        .unwrap();
    let document = files::document(&files, change, &Cancel::default()).unwrap();
    Snapshot::capture(&files, change, &document, &Cancel::default()).unwrap()
}
#[test]
fn reading_a_partial_snapshot_preserves_other_dirty_files_and_the_real_index() {
    let f = Fixture::create();
    f.commit("file.txt", "base\n");
    f.commit("other.txt", "old\n");
    write(f.repo.join("file.txt"), "changed\n");
    write(f.repo.join("other.txt"), "uncommitted external change\n");
    let index = fs::read(f.repo.join(".git/index")).unwrap();
    let _snapshot = snapshot(&f.repo, "file.txt", Area::Unstaged);
    assert_eq!(fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(
        fs::read_to_string(f.repo.join("other.txt")).unwrap(),
        "uncommitted external change\n"
    );
    assert_eq!(
        fs::read_to_string(f.repo.join("file.txt")).unwrap(),
        "changed\n"
    );
}
#[test]
fn an_unborn_repository_can_partially_stage_and_fully_unstage_its_first_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("unborn");
    init(&path);
    write(path.join("first.txt"), "one\ntwo\n");
    let source = snapshot(&path, "first.txt", Area::Untracked);
    let plan = source
        .prepare(
            Action::Stage,
            [LineId::New(1.try_into().unwrap())].into_iter().collect(),
        )
        .unwrap();
    partial::execute(&plan, &Cancel::default()).unwrap();
    assert_eq!(git(&path, &["show", ":first.txt"]), "one");
    let source = snapshot(&path, "first.txt", Area::Staged);
    let plan = source
        .prepare(
            Action::Unstage,
            source.hunks().iter().flatten().copied().collect(),
        )
        .unwrap();
    partial::execute(&plan, &Cancel::default()).unwrap();
    assert!(
        engine::local::open(&path)
            .unwrap()
            .index()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        fs::read_to_string(path.join("first.txt")).unwrap(),
        "one\ntwo\n"
    );
}
#[test]
fn discard_restores_deleted_parent_directories_and_preserves_restrictive_permissions() {
    let f = Fixture::create();
    fs::create_dir(f.repo.join("nested")).unwrap();
    f.commit("nested/file.txt", "one\ntwo\n");
    fs::remove_dir_all(f.repo.join("nested")).unwrap();
    let source = snapshot(&f.repo, "nested/file.txt", Area::Unstaged);
    let plan = source
        .prepare(
            Action::Discard,
            [LineId::Old(2.try_into().unwrap())].into_iter().collect(),
        )
        .unwrap();
    partial::execute(&plan, &Cancel::default()).unwrap();
    assert_eq!(
        fs::read_to_string(f.repo.join("nested/file.txt")).unwrap(),
        "two\n"
    );
    fs::set_permissions(
        f.repo.join("nested/file.txt"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let source = snapshot(&f.repo, "nested/file.txt", Area::Unstaged);
    let plan = source
        .prepare(
            Action::Discard,
            source.hunks().iter().flatten().copied().collect(),
        )
        .unwrap();
    partial::execute(&plan, &Cancel::default()).unwrap();
    assert_eq!(
        fs::metadata(f.repo.join("nested/file.txt"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}
#[test]
fn a_replaced_parent_symlink_and_cancelled_plan_never_write_outside_the_repository() {
    let f = Fixture::create();
    fs::create_dir(f.repo.join("nested")).unwrap();
    f.commit("nested/file.txt", "base\n");
    write(f.repo.join("nested/file.txt"), "changed\n");
    let source = snapshot(&f.repo, "nested/file.txt", Area::Unstaged);
    let plan = source
        .prepare(
            Action::Discard,
            source.hunks().iter().flatten().copied().collect(),
        )
        .unwrap();
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(partial::execute(&plan, &cancel).is_err());
    assert_eq!(
        fs::read_to_string(f.repo.join("nested/file.txt")).unwrap(),
        "changed\n"
    );
    let outside = tempfile::tempdir().unwrap();
    write(outside.path().join("file.txt"), "changed\n");
    fs::remove_dir_all(f.repo.join("nested")).unwrap();
    symlink(outside.path(), f.repo.join("nested")).unwrap();
    assert!(partial::execute(&plan, &Cancel::default()).is_err());
    assert_eq!(
        fs::read_to_string(outside.path().join("file.txt")).unwrap(),
        "changed\n"
    );
    assert!(!f.repo.join(".git/index.lock").exists());
}

#[test]
fn partial_lines_cannot_glue_a_new_line_onto_an_unterminated_existing_line() {
    let f = Fixture::create();
    f.commit("file.txt", "old");
    write(f.repo.join("file.txt"), "old\nnew\n");
    let source = snapshot(&f.repo, "file.txt", Area::Unstaged);
    let index = fs::read(f.repo.join(".git/index")).unwrap();
    assert!(
        source
            .prepare(
                Action::Stage,
                [LineId::New(2.try_into().unwrap())].into_iter().collect()
            )
            .is_err()
    );
    assert_eq!(fs::read(f.repo.join(".git/index")).unwrap(), index);
    let plan = source
        .prepare(
            Action::Stage,
            source.hunks().iter().flatten().copied().collect(),
        )
        .unwrap();
    partial::execute(&plan, &Cancel::default()).unwrap();
    assert_eq!(git(&f.repo, &["show", ":file.txt"]), "old\nnew");
}

#[test]
fn discard_preview_identifies_full_file_creation_and_deletion_without_applying_it() {
    let f = Fixture::create();
    write(f.repo.join("new.txt"), "new\n");
    let source = snapshot(&f.repo, "new.txt", Area::Untracked);
    let plan = source
        .prepare(
            Action::Discard,
            source.hunks().iter().flatten().copied().collect(),
        )
        .unwrap();
    let preview = plan.preview().unwrap();
    assert!(
        preview.contains("deleted file mode") && preview.contains("+++ /dev/null"),
        "{preview}"
    );
    assert_eq!(fs::read_to_string(f.repo.join("new.txt")).unwrap(), "new\n");
    f.commit("old.txt", "old\n");
    fs::remove_file(f.repo.join("old.txt")).unwrap();
    let source = snapshot(&f.repo, "old.txt", Area::Unstaged);
    let plan = source
        .prepare(
            Action::Discard,
            source.hunks().iter().flatten().copied().collect(),
        )
        .unwrap();
    let preview = plan.preview().unwrap();
    assert!(
        preview.contains("new file mode") && preview.contains("--- /dev/null"),
        "{preview}"
    );
    assert!(!f.repo.join("old.txt").exists());
}

#[test]
fn capture_rejects_mismatched_displayed_text_and_external_checkout_filters() {
    let f = Fixture::create();
    f.commit("file.txt", "old\n");
    write(f.repo.join("file.txt"), "new\n");
    let files = files::read(&f.repo).unwrap();
    let change = files
        .changes
        .iter()
        .find(|change| change.location.name() == Path::new("file.txt"))
        .unwrap();
    let mut document = files::document(&files, change, &Cancel::default()).unwrap();
    let files::patch::Document::Patch { files: patches, .. } = &mut document else {
        panic!("text patch");
    };
    for line in &mut patches[0].lines {
        if let files::patch::Line::Added { text, .. } = line {
            *text = "stale displayed text\n".into();
        }
    }
    assert!(Snapshot::capture(&files, change, &document, &Cancel::default()).is_err());
    write(f.repo.join(".gitattributes"), "file.txt filter=external\n");
    git(&f.repo, &["config", "filter.external.smudge", "cat"]);
    let files = files::read(&f.repo).unwrap();
    let change = files
        .changes
        .iter()
        .find(|change| change.location.name() == Path::new("file.txt"))
        .unwrap();
    let document = files::document(&files, change, &Cancel::default()).unwrap();
    assert!(Snapshot::capture(&files, change, &document, &Cancel::default()).is_err());
    assert_eq!(
        fs::read_to_string(f.repo.join("file.txt")).unwrap(),
        "new\n"
    );
}
