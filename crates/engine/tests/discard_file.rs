use engine::{
    cancel::Cancel,
    local::files::{
        self, Area,
        discard::{Action, Plan},
    },
};
#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};
use std::{fs, path::Path};
use test_support::*;

fn plan(f: &Fixture, name: &str, area: Area) -> Plan {
    let files = files::read(&f.repo).unwrap();
    let change = files
        .changes
        .iter()
        .find(|change| change.area == area && change.location.name() == Path::new(name))
        .unwrap()
        .clone();
    Plan::prepare(files, &[change], &Cancel::default()).unwrap()
}

#[test]
fn whole_file_restore_keeps_index_and_other_files_and_supports_binary_and_deleted_entries() {
    let f = Fixture::create();
    let name = "子目录/带空格.bin";
    fs::create_dir(f.repo.join("子目录")).unwrap();
    f.commit(name, "HEAD\n");
    let file = f.repo.join(name);
    let index_file = f.repo.join(".git/index");
    fs::write(&file, b"\0index\xff").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
    git(&f.repo, &["add", "--", name]);
    fs::write(&file, b"\0worktree\x80").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    write(f.repo.join("unrelated.txt"), "keep");
    let index = fs::read(&index_file).unwrap();
    let restore = plan(&f, name, Area::Unstaged);
    assert_eq!(
        restore.entries().next(),
        Some((Path::new(name), Action::Restore))
    );
    restore.execute(&Cancel::default()).unwrap();
    assert_eq!(fs::read(&file).unwrap(), b"\0index\xff");
    #[cfg(unix)]
    assert_ne!(fs::metadata(&file).unwrap().permissions().mode() & 0o100, 0);
    assert_eq!(fs::read(&index_file).unwrap(), index);

    fs::remove_file(&file).unwrap();
    fs::remove_dir(f.repo.join("子目录")).unwrap();
    plan(&f, name, Area::Unstaged)
        .execute(&Cancel::default())
        .unwrap();
    assert_eq!(fs::read(&file).unwrap(), b"\0index\xff");
    assert_eq!(fs::read(&index_file).unwrap(), index);
    assert_eq!(
        fs::read_to_string(f.repo.join("unrelated.txt")).unwrap(),
        "keep"
    );
}

#[cfg(unix)]
#[test]
fn whole_file_discard_rejects_edits_and_removes_only_a_reviewed_untracked_symlink() {
    let f = Fixture::create();
    f.commit("tracked.txt", "base\n");
    let file = f.repo.join("tracked.txt");
    let untracked = f.repo.join("untracked");
    write(&file, "worktree\n");
    let restore = plan(&f, "tracked.txt", Area::Unstaged);
    write(&file, "later edit\n");
    assert!(restore.execute(&Cancel::default()).is_err());
    assert_eq!(fs::read_to_string(&file).unwrap(), "later edit\n");

    symlink("tracked.txt", &untracked).unwrap();
    let remove = plan(&f, "untracked", Area::Untracked);
    assert_eq!(
        remove.entries().next(),
        Some((Path::new("untracked"), Action::RemoveUntracked))
    );
    git(&f.repo, &["add", "tracked.txt"]);
    assert!(remove.execute(&Cancel::default()).is_err());
    assert_eq!(fs::read_link(&untracked).unwrap(), Path::new("tracked.txt"));
    plan(&f, "untracked", Area::Untracked)
        .execute(&Cancel::default())
        .unwrap();
    assert!(!untracked.exists());
    assert_eq!(fs::read_to_string(&file).unwrap(), "later edit\n");
}

#[test]
fn batch_revalidates_every_file_before_writing_and_preserves_the_index() {
    let f = Fixture::create();
    f.commit("a.txt", "base a\n");
    f.commit("b.txt", "base b\n");
    let a = f.repo.join("a.txt");
    let b = f.repo.join("b.txt");
    let untracked = f.repo.join("untracked");
    write(&a, "staged a\n");
    git(&f.repo, &["add", "a.txt"]);
    let index_path = f.repo.join(".git/index");
    let index = fs::read(&index_path).unwrap();
    write(&a, "worktree a\n");
    write(&b, "worktree b\n");
    write(&untracked, "untracked\n");

    let cancel = Cancel::default();
    let make_plan = || {
        let files = files::read(&f.repo).unwrap();
        let changes = files
            .changes
            .iter()
            .filter(|change| change.area != Area::Staged)
            .cloned()
            .collect::<Vec<_>>();
        Plan::prepare(files, &changes, &cancel).unwrap()
    };
    let first = make_plan();
    assert_eq!(first.entries().len(), 3);
    write(&b, "later edit\n");
    assert!(first.execute(&cancel).is_err());
    assert_eq!(fs::read_to_string(&a).unwrap(), "worktree a\n");
    assert_eq!(fs::read_to_string(&b).unwrap(), "later edit\n");
    assert!(untracked.exists());

    make_plan().execute(&cancel).unwrap();
    assert_eq!(fs::read_to_string(&a).unwrap(), "staged a\n");
    assert_eq!(fs::read_to_string(&b).unwrap(), "base b\n");
    assert!(!untracked.exists());
    assert_eq!(fs::read(index_path).unwrap(), index);
}
