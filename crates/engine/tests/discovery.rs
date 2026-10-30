use engine::{
    cancel::Cancel,
    discovery::{self, Record},
};
use std::{fs, os::unix::fs::symlink};
use test_support::*;
#[test]
fn repository_ignores_stop_traversal_and_tracked_paths_remain_visible() {
    let _language = i18n::using(i18n::Language::Chinese);
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    let parent = root.join("parent");
    init(&parent);
    write(parent.join(".gitignore"), "ignored/\ntracked/\n");
    let ignored = parent.join("ignored/deep");
    init(&ignored);
    let visible = parent.join("visible/nested");
    init(&visible);
    let tracked = parent.join("tracked");
    fs::create_dir(&tracked).unwrap();
    write(tracked.join("keep"), "tracked");
    git(&parent, &["add", "-f", "tracked/keep"]);
    let nested = tracked.join("child");
    init(&nested);
    let report = discovery::discover(&root, &[], &Cancel::default()).unwrap();
    let paths = report
        .repositories
        .iter()
        .map(|r| r.path().to_owned())
        .collect::<Vec<_>>();
    assert!(paths.contains(&parent));
    assert!(paths.contains(&visible));
    assert!(!paths.contains(&ignored));
    assert!(!paths.contains(&nested));
    let report = discovery::discover(&ignored, &[], &Cancel::default()).unwrap();
    assert_eq!(report.repositories.len(), 1);
}
#[test]
fn linked_submodule_exclusion_markers_and_symlinks() {
    let _language = i18n::using(i18n::Language::Chinese);
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    let parent = root.join("parent");
    init(&parent);
    git(&parent, &["commit", "--allow-empty", "-m", "base"]);
    let linked = root.join("linked");
    git(
        &parent,
        &["worktree", "add", "-b", "linked", linked.to_str().unwrap()],
    );
    let ignored = root.join("excluded/repo");
    init(&ignored);
    let other = root.join("marker");
    fs::create_dir(&other).unwrap();
    write(other.join(".git"), "ordinary file");
    let target = root.join("link");
    symlink(&parent, &target).unwrap();
    let report = discovery::discover(&root, &[root.join("excluded")], &Cancel::default()).unwrap();
    assert_eq!(report.repositories.len(), 2);
    assert!(
        report
            .repositories
            .iter()
            .all(|r| matches!(r, Record::Scanned { .. }))
    );
    assert_eq!(report.non_repository_markers, vec![other.join(".git")]);
}
#[test]
fn submodules_are_discovered_using_index_gitlinks() {
    let _language = i18n::using(i18n::Language::Chinese);
    let parent = Fixture::create();
    let sub = Fixture::create();
    git(
        &parent.repo,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            sub.remote.to_str().unwrap(),
            "sub",
        ],
    );
    let report = discovery::discover(&parent.repo, &[], &Cancel::default()).unwrap();
    assert_eq!(report.repositories.len(), 2);
    assert!(
        report
            .repositories
            .iter()
            .any(|r| r.path() == parent.repo.join("sub"))
    );
}

#[test]
fn nested_negation_global_exclusion_and_nested_repository_rules() {
    let _language = i18n::using(i18n::Language::Chinese);
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    let parent = root.join("parent");
    init(&parent);
    for name in [
        "cache/hidden",
        "packages/keep",
        "packages/drop",
        "global-hidden",
        "local-hidden",
        "nested/cache/hidden",
        "nested/visible",
    ] {
        init(&parent.join(name));
    }
    init(&parent.join("nested"));
    write(parent.join(".gitignore"), "cache/\n");
    write(parent.join("packages/.gitignore"), "*\n!keep/\n");
    write(parent.join(".git/info/exclude"), "local-hidden/\n");
    let global = root.join("global-ignore");
    write(&global, "global-hidden/\n");
    git(
        &parent,
        &["config", "core.excludesFile", global.to_str().unwrap()],
    );
    write(parent.join("nested/.gitignore"), "cache/\n");
    let report = discovery::discover(&root, &[], &Cancel::default()).unwrap();
    let paths = report
        .repositories
        .iter()
        .map(|r| r.path().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(paths.len(), 4, "{paths:?}");
    for name in ["packages/keep", "nested", "nested/visible"] {
        assert!(paths.contains(&parent.join(name)))
    }
    let broken = root.join("broken");
    fs::create_dir(&broken).unwrap();
    write(broken.join(".git"), "gitdir: /path/to/unavailable/gitdir\n");
    init(&broken.join("nested"));
    let report = discovery::discover(&broken, &[], &Cancel::default()).unwrap();
    assert!(matches!(
        report.repositories.as_slice(),
        [Record::Failed { .. }]
    ));
}

#[test]
fn scan_request_accepts_many_exclusions_after_one_flag_and_expands_only_home() {
    let args = ["~/Proj", "--exclude", "backup", "归档 目录", "~/Proj/other"].map(str::to_owned);
    let request =
        discovery::Request::from_args(&args, std::path::Path::new("/users/test")).unwrap();
    assert_eq!(request.root, std::path::Path::new("/users/test/Proj"));
    assert_eq!(
        request.excluded,
        [
            std::path::PathBuf::from("backup"),
            "归档 目录".into(),
            "/users/test/Proj/other".into()
        ]
    );
    for args in [
        vec![],
        vec!["/tmp".into(), "--exclude".into()],
        vec!["/tmp".into(), "--unknown".into(), "a".into()],
    ] {
        assert!(discovery::Request::from_args(&args, std::path::Path::new("/users/test")).is_err());
    }
}

#[test]
fn service_scan_exclusions_are_temporary_and_do_not_save_reports_or_restrict_registration() {
    use engine::{scheduler::Manager, service::Service, state::Store};
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    let projects = root.join("项目 目录");
    for path in ["keep", "backup/old", "中文 归档/old"] {
        init(&projects.join(path));
    }
    let store = Store::custom(root.join("state"));
    store.initialize().unwrap();
    let service = Service::new(
        store.clone(),
        Manager::for_user(store.config_dir(), unsafe { libc::getuid() }).unwrap(),
    );
    let before = fs::read(store.config_path()).unwrap();
    let request = discovery::Request {
        root: projects.clone(),
        excluded: vec!["backup".into(), projects.join("中文 归档")],
    };
    let report = service.scan(&request, &Cancel::default()).unwrap();
    assert_eq!(report.repositories.len(), 1);
    assert_eq!(report.repositories[0].path(), projects.join("keep"));
    assert_eq!(fs::read(store.config_path()).unwrap(), before);
    assert!(store.scan_report().unwrap().is_none());
    let unfiltered = service
        .scan(
            &discovery::Request {
                root: projects.clone(),
                excluded: vec![],
            },
            &Cancel::default(),
        )
        .unwrap();
    assert_eq!(unfiltered.repositories.len(), 3);
    assert_eq!(service.add(&[projects.join("backup/old")]).unwrap(), 1);
    assert_eq!(
        store.config().unwrap().repositories,
        [projects.join("backup/old")]
    );
    let cancelled = Cancel::default();
    cancelled.cancel();
    assert!(service.scan(&request, &cancelled).is_err());
    assert!(discovery::discover(&projects, &[root.join("state")], &Cancel::default()).is_err());
}
