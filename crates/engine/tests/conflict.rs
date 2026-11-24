use engine::{
    cancel::Cancel,
    local::files::{
        self,
        conflict::{self, Cell, Comparison, Version},
        patch::{Source, SourceOmission},
    },
};
use std::{fs, path::Path};
use test_support::{Fixture, git, write};

fn failed_git(path: &Path, args: &[&str]) {
    let output = test_support::git_command()
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", test_support::NULL_DEVICE)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn version(f: &Fixture, name: &str, text: Option<&[u8]>, message: &str) {
    match text {
        Some(bytes) => fs::write(f.repo.join(name), bytes).unwrap(),
        None => fs::remove_file(f.repo.join(name)).unwrap(),
    }
    git(&f.repo, &["add", "--all"]);
    git(&f.repo, &["commit", "-m", message]);
}
fn conflict(base: Option<&[u8]>, current: Option<&[u8]>, incoming: &[u8]) -> (Fixture, String) {
    let f = Fixture::create();
    let path = "中文 空格[1].ts";
    if let Some(base) = base {
        version(&f, path, Some(base), "base");
    }
    git(&f.repo, &["checkout", "-b", "incoming"]);
    version(&f, path, Some(incoming), "incoming");
    let oid = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["checkout", "main"]);
    version(&f, path, current, "current");
    failed_git(&f.repo, &["cherry-pick", &oid]);
    (f, path.into())
}
fn read(f: &Fixture, name: &str) -> conflict::Snapshot {
    let files = files::read(&f.repo).unwrap();
    let change = files
        .changes
        .iter()
        .find(|c| c.location.name() == Path::new(name))
        .unwrap();
    let index = fs::read(f.repo.join(".git/index")).unwrap();
    let worktree = fs::read(f.repo.join(name)).ok();
    let head = git(&f.repo, &["rev-parse", "HEAD"]);
    let sequencer = fs::read(f.repo.join(".git/CHERRY_PICK_HEAD")).ok();
    let result = conflict::read(&files, change, &Cancel::default()).unwrap();
    assert_eq!(fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(f.repo.join(name)).ok(), worktree);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        fs::read(f.repo.join(".git/CHERRY_PICK_HEAD")).ok(),
        sequencer
    );
    result
}
#[test]
fn index_stages_are_exact_literal_path_snapshots_and_never_worktree_markers() {
    let (f, name) = conflict(Some(b"base\n"), Some(b"current\n"), b"incoming\n");
    write(f.repo.join(&name), "external edit without markers\n");
    let comparison = read(&f, &name);
    assert_eq!(comparison.base.text(), Some("base\n"));
    assert_eq!(comparison.current.text(), Some("current\n"));
    assert_eq!(comparison.incoming.text(), Some("incoming\n"));
    let Comparison::Text(rows) = comparison.comparison else {
        panic!("text");
    };
    assert_eq!(
        (rows[0].base, rows[0].current, rows[0].incoming),
        (Cell::Changed(0), Cell::Changed(0), Cell::Changed(0))
    );
}
#[test]
fn add_add_and_modify_delete_keep_absent_stages_explicit() {
    let (f, name) = conflict(None, Some(b"ours\n"), b"theirs\n");
    let added = read(&f, &name);
    assert!(matches!(added.base, Version::Absent));
    assert_eq!(added.current.text(), Some("ours\n"));
    let (f, name) = conflict(Some(b"base\n"), None, b"theirs\n");
    let deleted = read(&f, &name);
    assert!(matches!(deleted.current, Version::Absent));
    assert_eq!(deleted.base.text(), Some("base\n"));
    assert_eq!(deleted.incoming.text(), Some("theirs\n"));
}
#[test]
fn binary_stages_are_explicit_and_resolving_the_index_invalidates_the_snapshot() {
    let (f, name) = conflict(Some(b"base\0"), Some(b"current\0"), b"incoming\0");
    let comparison = read(&f, &name);
    assert!(matches!(comparison.comparison, Comparison::NonText));
    assert!(matches!(
        comparison.current,
        Version::Present {
            source: Source::Unavailable(SourceOmission::Binary),
            ..
        }
    ));
    let files = files::read(&f.repo).unwrap();
    let change = files.changes[0].clone();
    git(&f.repo, &["add", "--", &name]);
    assert!(conflict::read(&files, &change, &Cancel::default()).is_err());
}
#[test]
fn revert_uses_the_reverted_commit_as_base_and_its_parent_as_incoming() {
    let f = Fixture::create();
    f.commit("f.txt", "base\n");
    f.commit("f.txt", "feature\n");
    let feature = git(&f.repo, &["rev-parse", "HEAD"]);
    f.commit("f.txt", "later\n");
    failed_git(&f.repo, &["revert", "--no-edit", &feature]);
    let comparison = read(&f, "f.txt");
    assert_eq!(comparison.base.text(), Some("feature\n"));
    assert_eq!(comparison.current.text(), Some("later\n"));
    assert_eq!(comparison.incoming.text(), Some("base\n"));
}

#[test]
fn oversized_and_nonregular_index_versions_remain_distinct_from_missing_files() {
    let (f, name) = conflict(Some(b"base\n"), Some(b"current\n"), b"incoming\n");
    let repo = git2::Repository::open(&f.repo).unwrap();
    let mut index = repo.index().unwrap();
    let mut entry = index.get_path(Path::new(&name), 2).unwrap();
    entry.id = repo
        .blob(&vec![b'x'; files::patch::MAX_SOURCE_BYTES + 1])
        .unwrap();
    index.add(&entry).unwrap();
    index.write().unwrap();
    let large = read(&f, &name);
    assert!(matches!(
        large.current,
        Version::Present {
            source: Source::Unavailable(SourceOmission::TooLarge),
            ..
        }
    ));
    entry.id = repo.blob(b"actual link target").unwrap();
    entry.mode = 0o120000;
    index.add(&entry).unwrap();
    index.write().unwrap();
    let link = read(&f, &name);
    assert!(matches!(
        link.current,
        Version::Present {
            mode: 0o120000,
            source: Source::Unavailable(SourceOmission::NonRegular),
            ..
        }
    ));
}

#[cfg(unix)]
#[test]
fn working_result_is_independent_of_patch_presence_and_keeps_non_text_states_explicit() {
    let (f, name) = conflict(Some(b"base\n"), Some(b"current\n"), b"incoming\n");
    let snapshot = files::read(&f.repo).unwrap();
    let change = snapshot
        .changes
        .iter()
        .find(|change| change.kind == 'U')
        .unwrap();
    let index = fs::read(f.repo.join(".git/index")).unwrap();
    let path = f.repo.join(&name);
    write(&path, "current\n");
    let source = conflict::working(&snapshot, change, &Cancel::default()).unwrap();
    assert!(
        matches!(source, Source::Text { path, text } if path == Path::new(&name) && text == "current\n")
    );
    fs::write(&path, b"binary\0result").unwrap();
    assert!(matches!(
        conflict::working(&snapshot, change, &Cancel::default()).unwrap(),
        Source::Unavailable(SourceOmission::Binary)
    ));
    fs::remove_file(&path).unwrap();
    assert!(matches!(
        conflict::working(&snapshot, change, &Cancel::default()).unwrap(),
        Source::Absent
    ));
    std::os::unix::fs::symlink("outside", &path).unwrap();
    assert!(matches!(
        conflict::working(&snapshot, change, &Cancel::default()).unwrap(),
        Source::Unavailable(SourceOmission::NonRegular)
    ));
    assert_eq!(fs::read(f.repo.join(".git/index")).unwrap(), index);
}
