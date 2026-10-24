use super::*;
use test_support as support;

#[test]
fn lazy_directories_include_tracked_files_inside_ignored_parents_and_exclude_git_metadata() {
    let f = support::Fixture::create();
    std::fs::create_dir_all(f.repo.join("ignored")).unwrap();
    f.commit("ignored/keep.txt", "tracked\n");
    write(f.repo.join(".gitignore"), "ignored/\n*.log\n");
    write(f.repo.join(".git/info/exclude"), "excluded.txt\n");
    write(f.repo.join("ignored/hidden.txt"), "ignored\n");
    write(
        f.repo.join("deep/中文 目录/file.ts"),
        "export const a = 1;\n",
    );
    write(f.repo.join("excluded.txt"), "excluded\n");
    write(f.repo.join("error.log"), "ignored\n");
    let index = fs::read(f.repo.join(".git/index")).unwrap();
    let root = directory(&f.repo, Path::new(""), &Cancel::default()).unwrap();
    let paths = root
        .entries
        .iter()
        .map(|entry| entry.path.as_path())
        .collect::<Vec<_>>();
    assert!(paths.contains(&Path::new("ignored")));
    assert!(paths.contains(&Path::new("deep")));
    assert!(!paths.contains(&Path::new("deep/中文 目录/file.ts")));
    for hidden in [".git", "error.log", "excluded.txt"] {
        assert!(!paths.contains(&Path::new(hidden)));
    }
    let ignored = directory(&f.repo, Path::new("ignored"), &Cancel::default()).unwrap();
    assert_eq!(ignored.entries.len(), 1);
    assert_eq!(ignored.entries[0].path, Path::new("ignored/keep.txt"));
    assert_eq!(fs::read(f.repo.join(".git/index")).unwrap(), index);
}

#[test]
fn statuses_preserve_index_worktree_changes_deletions_and_detached_head() {
    let f = support::Fixture::create();
    f.commit("tracked.txt", "initial\n");
    f.commit("gone.txt", "initial\n");
    support::git(&f.repo, &["checkout", "--detach"]);
    write(f.repo.join("tracked.txt"), "staged\n");
    support::git(&f.repo, &["add", "tracked.txt"]);
    write(f.repo.join("tracked.txt"), "worktree\n");
    fs::remove_file(f.repo.join("gone.txt")).unwrap();
    write(f.repo.join("new.txt"), "new\n");
    let entries = directory(&f.repo, Path::new(""), &Cancel::default())
        .unwrap()
        .entries;
    let status = |path: &str| {
        entries
            .iter()
            .find(|entry| entry.path == Path::new(path))
            .unwrap()
            .status()
            .unwrap()
    };
    assert_eq!(status("tracked.txt").label(), "MM");
    assert_eq!(status("gone.txt").label(), " D");
    assert_eq!(status("new.txt"), FileStatus::Untracked);
    assert!(matches!(
        read(&f.repo, Path::new("gone.txt"), &Cancel::default()).unwrap(),
        Content::Missing
    ));
}

#[test]
fn previews_and_search_do_not_follow_symlinks_or_include_ignored_files() {
    use std::os::unix::fs::symlink;
    let f = support::Fixture::create();
    write(f.repo.join(".gitignore"), "ignored/\n");
    write(
        f.repo.join("deep/中文 目录/source.ts"),
        "export const value = 1;\n",
    );
    write(f.repo.join("ignored/source.ts"), "ignored\n");
    let outside = tempfile::tempdir().unwrap();
    write(outside.path().join("private.txt"), "outside\n");
    symlink(outside.path(), f.repo.join("link")).unwrap();
    assert!(matches!(
        read(&f.repo, Path::new("link"), &Cancel::default()).unwrap(),
        Content::Symlink(_)
    ));
    assert!(directory(&f.repo, Path::new("link"), &Cancel::default()).is_err());
    assert!(read(&f.repo, Path::new("link/private.txt"), &Cancel::default()).is_err());
    assert!(read(&f.repo, Path::new("../private.txt"), &Cancel::default()).is_err());
    let found = find(&f.repo, "source", &Cancel::default()).unwrap();
    assert_eq!(found.extent, SearchExtent::Complete);
    assert_eq!(found.entries.len(), 1);
    assert_eq!(found.entries[0].path, Path::new("deep/中文 目录/source.ts"));
    let Content::Text { text, .. } =
        read(&f.repo, &found.entries[0].path, &Cancel::default()).unwrap()
    else {
        panic!("text")
    };
    assert_eq!(text, "export const value = 1;\n");
    fs::write(f.repo.join("binary"), b"a\0b").unwrap();
    assert!(matches!(
        read(&f.repo, Path::new("binary"), &Cancel::default()).unwrap(),
        Content::Binary
    ));
    fs::write(
        f.repo.join("large"),
        vec![b'a'; super::super::files::patch::MAX_SOURCE_BYTES + 1],
    )
    .unwrap();
    assert!(matches!(
        read(&f.repo, Path::new("large"), &Cancel::default()).unwrap(),
        Content::TooLarge
    ));
}

#[test]
fn searches_report_the_match_limit_and_can_be_cancelled() {
    let f = support::Fixture::create();
    for i in 0..1001 {
        write(f.repo.join(format!("match-{i}")), "test\n");
    }
    let found = find(&f.repo, "match", &Cancel::default()).unwrap();
    assert_eq!(found.extent, SearchExtent::Limited);
    assert_eq!(found.entries.len(), 1000);
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(directory(&f.repo, Path::new(""), &cancel).is_err());
    assert!(find(&f.repo, "match", &cancel).is_err());
}

fn write(path: impl AsRef<std::path::Path>, text: &str) {
    let path = path.as_ref();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    support::write(path, text);
}

#[test]
fn submodules_are_leaf_entries_and_local_global_ignore_configuration_is_respected() {
    let f = support::Fixture::create();
    let nested = f.repo.join("nested");
    support::init(&nested);
    write(nested.join("child.txt"), "child\n");
    support::git(&nested, &["add", "."]);
    support::git(&nested, &["commit", "-m", "child"]);
    let oid = support::git(&nested, &["rev-parse", "HEAD"]);
    support::git(
        &f.repo,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{oid},nested"),
        ],
    );
    let ignore = f.repo.parent().unwrap().join("global-ignore");
    write(&ignore, "*.generated\n");
    support::git(
        &f.repo,
        &["config", "core.excludesFile", ignore.to_str().unwrap()],
    );
    write(f.repo.join("hidden.generated"), "hidden\n");
    let root = directory(&f.repo, Path::new(""), &Cancel::default()).unwrap();
    assert!(
        root.entries
            .iter()
            .any(|entry| entry.path == Path::new("nested")
                && matches!(entry.node, Node::Submodule(_)))
    );
    assert!(
        !root
            .entries
            .iter()
            .any(|entry| entry.path == Path::new("hidden.generated"))
    );
}
