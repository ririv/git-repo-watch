use engine::{
    cancel::Cancel,
    local::files::{
        self,
        conflict::resolve::{self, Choice, Snapshot},
        patch::Source,
    },
};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};
use test_support::{Fixture, git};

const PATH: &str = "src/中文 空格.ts";
const TWO: &str = "prefix\n<<<<<<< HEAD\nleft one\n=======\nright one\n>>>>>>> incoming\nunchanged\n<<<<<<< HEAD\nleft two\n=======\nright two\n>>>>>>> incoming\nsuffix";
fn fixture(source: &str) -> Fixture {
    let f = Fixture::create();
    fs::create_dir(f.repo.join("src")).unwrap();
    f.commit(PATH, "base\n");
    git(&f.repo, &["checkout", "-b", "incoming"]);
    f.commit(PATH, "incoming\n");
    let incoming = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["checkout", "main"]);
    f.commit(PATH, "current\n");
    assert_eq!(
        test_support::git_command()
            .arg("-C")
            .arg(&f.repo)
            .args(["cherry-pick", &incoming])
            .output()
            .unwrap()
            .status
            .code(),
        Some(1)
    );
    fs::write(f.repo.join(PATH), source).unwrap();
    f
}
fn capture(f: &Fixture) -> anyhow::Result<Snapshot> {
    let files = files::read(&f.repo)?;
    let change = files.changes.iter().find(|c| c.kind == 'U').unwrap();
    Snapshot::capture(
        &files,
        change,
        &Source::Text {
            path: PATH.into(),
            text: fs::read_to_string(f.repo.join(PATH))?,
        },
        &Cancel::default(),
    )
}
fn immutable(f: &Fixture) -> (Vec<u8>, String, Vec<u8>) {
    (
        fs::read(f.repo.join(".git/index")).unwrap(),
        git(&f.repo, &["rev-parse", "HEAD"]),
        fs::read(f.repo.join(".git/CHERRY_PICK_HEAD")).unwrap(),
    )
}
#[test]
fn each_choice_changes_only_the_selected_block_and_preserves_permissions_and_git_state() {
    for (choice, selected) in [
        (Choice::Left, "left one\n"),
        (Choice::Right, "right one\n"),
        (Choice::Both, "left one\nright one\n"),
    ] {
        let f = fixture(TWO);
        #[cfg(unix)]
        fs::set_permissions(f.repo.join(PATH), fs::Permissions::from_mode(0o755)).unwrap();
        let before = immutable(&f);
        let snapshot = capture(&f).unwrap();
        assert_eq!(snapshot.blocks().len(), 2);
        let plan = snapshot.prepare(0, choice).unwrap();
        resolve::execute(&plan, &Cancel::default()).unwrap();
        let expected = TWO.replacen(
            "<<<<<<< HEAD\nleft one\n=======\nright one\n>>>>>>> incoming\n",
            selected,
            1,
        );
        assert_eq!(fs::read_to_string(f.repo.join(PATH)).unwrap(), expected);
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(f.repo.join(PATH))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        assert_eq!(immutable(&f), before);
        assert!(!f.repo.join(".git/index.lock").exists());
    }
}
#[test]
fn diff3_custom_markers_crlf_and_empty_sides_preserve_the_actual_selected_bytes() {
    for (source, attributes, choice, expected) in [
        (
            "p\r\n<<<<<<< HEAD\r\nleft\r\n||||||| base\r\nancestor\r\n=======\r\nright\r\n>>>>>>> branch\r\ntail",
            None,
            Choice::Both,
            "p\r\nleft\r\nright\r\ntail",
        ),
        (
            "<<<<<<<<<< HEAD\nleft\n==========\n>>>>>>>>>> branch\n",
            Some("*.ts conflict-marker-size=10\n"),
            Choice::Right,
            "",
        ),
        (
            "=======\n<<<<<<< HEAD\n=======\nright\n>>>>>>> branch\n",
            None,
            Choice::Left,
            "=======\n",
        ),
    ] {
        let f = fixture(source);
        if let Some(attributes) = attributes {
            fs::write(f.repo.join(".gitattributes"), attributes).unwrap();
        }
        let snapshot = capture(&f).unwrap();
        assert_eq!(snapshot.blocks().len(), 1);
        resolve::execute(&snapshot.prepare(0, choice).unwrap(), &Cancel::default()).unwrap();
        assert_eq!(fs::read_to_string(f.repo.join(PATH)).unwrap(), expected);
    }
}
#[test]
fn guarded_undo_restores_one_resolution_and_refuses_changed_content_or_index() {
    let f = fixture(TWO);
    let plan = capture(&f).unwrap().prepare(1, Choice::Both).unwrap();
    resolve::execute(&plan, &Cancel::default()).unwrap();
    let undo = plan.inverse();
    resolve::execute(&undo, &Cancel::default()).unwrap();
    assert_eq!(fs::read_to_string(f.repo.join(PATH)).unwrap(), TWO);
    resolve::execute(&plan, &Cancel::default()).unwrap();
    fs::write(f.repo.join(PATH), "external editor saved new content\n").unwrap();
    assert!(resolve::execute(&undo, &Cancel::default()).is_err());
    assert_eq!(
        fs::read_to_string(f.repo.join(PATH)).unwrap(),
        "external editor saved new content\n"
    );
    fs::write(f.repo.join(PATH), TWO).unwrap();
    let plan = capture(&f).unwrap().prepare(0, Choice::Left).unwrap();
    git(&f.repo, &["add", "--", PATH]);
    assert!(resolve::execute(&plan, &Cancel::default()).is_err());
    assert_eq!(fs::read_to_string(f.repo.join(PATH)).unwrap(), TWO);
}
#[cfg(unix)]
#[test]
fn stale_views_locks_cancellation_and_symlink_replacements_never_overwrite_files() {
    let f = fixture(TWO);
    let plan = capture(&f).unwrap().prepare(0, Choice::Left).unwrap();
    fs::write(f.repo.join(".git/index.lock"), "other process").unwrap();
    assert!(resolve::execute(&plan, &Cancel::default()).is_err());
    assert_eq!(
        fs::read_to_string(f.repo.join(".git/index.lock")).unwrap(),
        "other process"
    );
    fs::remove_file(f.repo.join(".git/index.lock")).unwrap();
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(resolve::execute(&plan, &cancel).is_err());
    assert_eq!(fs::read_to_string(f.repo.join(PATH)).unwrap(), TWO);
    let outside = f.temp.path().join("outside.txt");
    fs::write(&outside, "outside content").unwrap();
    fs::remove_file(f.repo.join(PATH)).unwrap();
    symlink(&outside, f.repo.join(PATH)).unwrap();
    assert!(resolve::execute(&plan, &Cancel::default()).is_err());
    assert_eq!(fs::read_to_string(outside).unwrap(), "outside content");
    assert!(
        fs::symlink_metadata(f.repo.join(PATH))
            .unwrap()
            .is_symlink()
    );
}
#[test]
fn incomplete_or_nested_markers_are_rejected_and_manual_resolution_has_no_pending_blocks() {
    for source in [
        "<<<<<<< HEAD\nleft\n",
        "<<<<<<< HEAD\n<<<<<<< nested\n=======\nright\n>>>>>>> branch\n",
    ] {
        let f = fixture(source);
        assert!(capture(&f).is_err());
        assert_eq!(fs::read_to_string(f.repo.join(PATH)).unwrap(), source);
    }
    let f = fixture("manually resolved\n");
    assert!(capture(&f).unwrap().blocks().is_empty());
}

fn reset_fixture(attributes: Option<&str>) -> Fixture {
    let f = Fixture::create();
    fs::create_dir(f.repo.join("src")).unwrap();
    if let Some(attributes) = attributes {
        f.commit(".gitattributes", attributes);
    }
    let gap = (0..16)
        .map(|n| format!("unchanged {n}\n"))
        .collect::<String>();
    let source = |header: &str, first: &str, second: &str, tail: &str| {
        format!("{header}\n{gap}{first}\n{gap}{second}\n{gap}{tail}\n")
    };
    f.commit(
        PATH,
        &source("base header", "base one", "base two", "base tail"),
    );
    git(&f.repo, &["checkout", "-b", "incoming"]);
    f.commit(
        PATH,
        &source("base header", "right one", "right two", "incoming tail"),
    );
    let incoming = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["checkout", "main"]);
    f.commit(
        PATH,
        &source("local header", "left one", "left two", "base tail"),
    );
    assert_eq!(
        test_support::git_command()
            .arg("-C")
            .arg(&f.repo)
            .args(["cherry-pick", &incoming])
            .output()
            .unwrap()
            .status
            .code(),
        Some(1)
    );
    assert_eq!(capture(&f).unwrap().blocks().len(), 2);
    f
}
#[test]
fn reset_recreates_every_conflict_from_index_and_preserves_nonconflicting_changes_and_git_state() {
    let f = reset_fixture(None);
    let before = immutable(&f);
    #[cfg(unix)]
    fs::set_permissions(f.repo.join(PATH), fs::Permissions::from_mode(0o755)).unwrap();
    for choice in [Choice::Left, Choice::Right] {
        resolve::execute(
            &capture(&f).unwrap().prepare(0, choice).unwrap(),
            &Cancel::default(),
        )
        .unwrap();
    }
    let resolved = fs::read_to_string(f.repo.join(PATH)).unwrap();
    assert!(capture(&f).unwrap().blocks().is_empty());
    let reset = capture(&f).unwrap().reset(&Cancel::default()).unwrap();
    let review = reset.reset_review().unwrap();
    assert!(review.contains("<<<<<<< HEAD") && review.contains("right one"));
    assert_eq!(immutable(&f), before);
    assert_eq!(fs::read_to_string(f.repo.join(PATH)).unwrap(), resolved);
    resolve::execute(&reset, &Cancel::default()).unwrap();
    let text = fs::read_to_string(f.repo.join(PATH)).unwrap();
    assert!(text.starts_with("local header\n") && text.ends_with("incoming tail\n"));
    assert!(
        text.contains("left one\n=======\nright one")
            && text.contains("left two\n=======\nright two")
    );
    assert_eq!(capture(&f).unwrap().blocks().len(), 2);
    #[cfg(unix)]
    assert_eq!(
        fs::metadata(f.repo.join(PATH))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    assert_eq!(immutable(&f), before);
    resolve::execute(&reset.inverse(), &Cancel::default()).unwrap();
    assert_eq!(fs::read_to_string(f.repo.join(PATH)).unwrap(), resolved);
    assert_eq!(immutable(&f), before);
}
#[test]
fn reset_honors_crlf_custom_markers_and_diff3_style() {
    let f = reset_fixture(Some("*.ts text eol=crlf conflict-marker-size=10\n"));
    git(&f.repo, &["config", "merge.conflictstyle", "diff3"]);
    let reset = capture(&f).unwrap().reset(&Cancel::default()).unwrap();
    resolve::execute(&reset, &Cancel::default()).unwrap();
    let text = fs::read_to_string(f.repo.join(PATH)).unwrap();
    assert!(text.contains("<<<<<<<<<< HEAD\r\n"), "{text:?}");
    assert!(text.contains("|||||||||| base\r\n"), "{text:?}");
    assert!(text.contains("==========\r\n"), "{text:?}");
    assert!(!text.replace("\r\n", "").contains('\n'));
    assert_eq!(capture(&f).unwrap().blocks().len(), 2);
}
#[test]
fn reset_rejects_stale_bytes_and_index_or_unsupported_merge_drivers() {
    for change_index in [false, true] {
        let f = reset_fixture(None);
        let reset = capture(&f).unwrap().reset(&Cancel::default()).unwrap();
        let external = "external editor content\n";
        fs::write(f.repo.join(PATH), external).unwrap();
        if change_index {
            git(&f.repo, &["add", "--", PATH]);
        }
        let before = immutable(&f);
        assert!(resolve::execute(&reset, &Cancel::default()).is_err());
        assert_eq!(fs::read_to_string(f.repo.join(PATH)).unwrap(), external);
        assert_eq!(immutable(&f), before);
    }
    let f = reset_fixture(None);
    fs::write(f.repo.join(".gitattributes"), "*.ts merge=custom\n").unwrap();
    let before = immutable(&f);
    assert!(capture(&f).unwrap().reset(&Cancel::default()).is_err());
    assert_eq!(immutable(&f), before);
}
