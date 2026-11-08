use engine::{
    cancel::Cancel,
    domain::PushPolicy,
    local::{
        branches::edit,
        stash::{self, SaveMode},
    },
    operations::mutation::{Change, Request},
    scheduler::Manager,
    service::Service,
    state::{Config, Store},
};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use test_support::{Fixture, git, write};
fn service(paths: Vec<PathBuf>) -> (tempfile::TempDir, Service) {
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().into());
    store
        .save_config(&Config {
            interval_seconds: 21600,
            schedule_anchor: Default::default(),
            repositories: paths,
            scheduled_actions: engine::workflow::Plan::from_policy(PushPolicy::Disabled),
            max_concurrency: 4,
            fetch_on_startup: false,
            auto_fetch: Default::default(),
            experimental_features: true,
            chinese_command_trigger: true,
            text_wrap: false,
            file_icons: true,
            interaction_mode: Default::default(),
            view: Default::default(),
            mouse: Default::default(),
            editor: Default::default(),
            ai: Default::default(),
        })
        .unwrap();
    let manager = Manager::for_user(home.path().into(), unsafe { libc::getuid() }).unwrap();
    (home, Service::new(store, manager))
}
fn branch(action: edit::Action, name: &str) -> Request {
    Request::Branch {
        action,
        name: name.into(),
    }
}
fn run(s: &Service, f: &Fixture, request: Request) -> Change {
    let cancel = Cancel::default();
    let plan = s.prepare_mutation(&f.repo, request, &cancel).unwrap();
    s.mutate(&plan, &cancel).unwrap()
}
#[test]
fn branches_create_without_switching_and_safe_switch_runs_the_post_checkout_hook() {
    let f = Fixture::create();
    f.commit("file", "before\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    run(&s, &f, branch(edit::Action::Create, "feature/中文"));
    assert_eq!(git(&f.repo, &["branch", "--show-current"]), "main");
    let hook = f.repo.join(".git/hooks/post-checkout");
    write(
        &hook,
        "#!/bin/sh\nprintf '%s\\n' \"$1\" \"$2\" \"$3\" > .hook-result\n",
    );
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o700)).unwrap();
    run(&s, &f, branch(edit::Action::Switch, "feature/中文"));
    assert_eq!(git(&f.repo, &["branch", "--show-current"]), "feature/中文");
    assert_eq!(
        fs::read_to_string(f.repo.join(".hook-result"))
            .unwrap()
            .lines()
            .last(),
        Some("1")
    );
    assert!(s.store.repository_report(&f.repo).unwrap().is_some());
    assert_eq!(s.store.operation_history(&f.repo, 10).unwrap().len(), 2);
}
#[test]
fn branch_review_protects_dirty_files_changed_targets_and_other_worktrees() {
    let f = Fixture::create();
    f.commit("file", "main\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    run(&s, &f, branch(edit::Action::Create, "other"));
    write(f.repo.join("file"), "dirty\n");
    assert!(
        s.prepare_mutation(
            &f.repo,
            branch(edit::Action::Switch, "other"),
            &Cancel::default()
        )
        .is_err()
    );
    assert_eq!(fs::read_to_string(f.repo.join("file")).unwrap(), "dirty\n");
    git(&f.repo, &["restore", "file"]);
    let plan = s
        .prepare_mutation(
            &f.repo,
            branch(edit::Action::Switch, "other"),
            &Cancel::default(),
        )
        .unwrap();
    git(&f.repo, &["branch", "-f", "other", "HEAD~1"]);
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(git(&f.repo, &["branch", "--show-current"]), "main");
    let linked = f.temp.path().join("linked");
    git(
        &f.repo,
        &["worktree", "add", linked.to_str().unwrap(), "other"],
    );
    assert!(
        s.prepare_mutation(
            &f.repo,
            branch(edit::Action::Switch, "other"),
            &Cancel::default()
        )
        .is_err()
    );
    assert!(
        s.prepare_mutation(
            &f.repo,
            branch(edit::Action::Create, "other"),
            &Cancel::default()
        )
        .is_err()
    );
    assert!(
        s.prepare_mutation(
            &f.repo,
            branch(edit::Action::Create, "--invalid"),
            &Cancel::default()
        )
        .is_err()
    );
}
#[test]
fn stash_modes_apply_pop_and_drop_preserve_their_distinct_effects() {
    let f = Fixture::create();
    f.commit("tracked", "before\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    write(f.repo.join("tracked"), "tracked change\n");
    write(f.repo.join("中文 new"), "new\n");
    run(
        &s,
        &f,
        Request::Stash(stash::Request::Save {
            mode: SaveMode::Tracked,
            message: "tracked only".into(),
        }),
    );
    assert_eq!(
        fs::read_to_string(f.repo.join("tracked")).unwrap(),
        "before\n"
    );
    assert!(f.repo.join("中文 new").exists());
    let first = s.stashes(&f.repo, &Cancel::default()).unwrap().entries[0].oid;
    run(&s, &f, Request::Stash(stash::Request::Apply(first)));
    assert_eq!(
        s.stashes(&f.repo, &Cancel::default())
            .unwrap()
            .entries
            .len(),
        1
    );
    git(&f.repo, &["restore", "tracked"]);
    run(&s, &f, Request::Stash(stash::Request::Pop(first)));
    assert!(
        s.stashes(&f.repo, &Cancel::default())
            .unwrap()
            .entries
            .is_empty()
    );
    run(
        &s,
        &f,
        Request::Stash(stash::Request::Save {
            mode: SaveMode::IncludeUntracked,
            message: "all".into(),
        }),
    );
    assert!(!f.repo.join("中文 new").exists());
    let second = s.stashes(&f.repo, &Cancel::default()).unwrap().entries[0].oid;
    run(&s, &f, Request::Stash(stash::Request::Drop(second)));
    assert!(!f.repo.join("中文 new").exists());
    assert!(
        s.stashes(&f.repo, &Cancel::default())
            .unwrap()
            .entries
            .is_empty()
    );
    assert_eq!(s.store.operation_history(&f.repo, 20).unwrap().len(), 5);
}
#[test]
fn stash_review_detects_content_and_reflog_changes_before_any_destructive_write() {
    let f = Fixture::create();
    f.commit("file", "base\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    write(f.repo.join("file"), "first\n");
    let request = Request::Stash(stash::Request::Save {
        mode: SaveMode::Tracked,
        message: "first".into(),
    });
    let plan = s
        .prepare_mutation(&f.repo, request.clone(), &Cancel::default())
        .unwrap();
    write(f.repo.join("file"), "later\n");
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(fs::read_to_string(f.repo.join("file")).unwrap(), "later\n");
    run(&s, &f, request);
    let oid = s.stashes(&f.repo, &Cancel::default()).unwrap().entries[0].oid;
    let plan = s
        .prepare_mutation(
            &f.repo,
            Request::Stash(stash::Request::Drop(oid)),
            &Cancel::default(),
        )
        .unwrap();
    write(f.repo.join("file"), "second\n");
    git(&f.repo, &["stash", "push", "-m", "external stash"]);
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(
        s.stashes(&f.repo, &Cancel::default())
            .unwrap()
            .entries
            .len(),
        2
    );
}

#[test]
fn pull_fetches_once_and_fast_forwards_the_exact_reviewed_upstream() {
    let f = Fixture::create();
    let (_home, s) = service(vec![f.repo.clone()]);
    let before = git(&f.repo, &["rev-parse", "HEAD"]);
    f.advance();
    let remote = git(&f.remote, &["rev-parse", "main"]);
    let plan = s
        .prepare_mutation(&f.repo, Request::Pull, &Cancel::default())
        .unwrap();
    assert!(!plan.current());
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), before);
    s.mutate(&plan, &Cancel::default()).unwrap();
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), remote);
    assert!(
        s.prepare_mutation(&f.repo, Request::Pull, &Cancel::default())
            .unwrap()
            .current()
    );
    assert_eq!(s.store.operation_history(&f.repo, 10).unwrap().len(), 3);
}

#[test]
fn pull_refuses_divergence_stale_reviews_and_deleted_upstreams_without_merging() {
    let f = Fixture::create();
    let (_home, s) = service(vec![f.repo.clone()]);
    f.advance();
    let plan = s
        .prepare_mutation(&f.repo, Request::Pull, &Cancel::default())
        .unwrap();
    git(&f.repo, &["commit", "--allow-empty", "-m", "local ahead"]);
    let local = git(&f.repo, &["rev-parse", "HEAD"]);
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert!(
        s.prepare_mutation(&f.repo, Request::Pull, &Cancel::default())
            .is_err()
    );
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), local);
    git(&f.remote, &["update-ref", "-d", "refs/heads/main"]);
    assert!(
        s.prepare_mutation(&f.repo, Request::Pull, &Cancel::default())
            .is_err()
    );
    assert!(
        git2::Repository::open(&f.repo)
            .unwrap()
            .find_reference("refs/remotes/origin/main")
            .is_err()
    );
}

#[test]
fn explicit_fetch_and_pull_use_custom_refspecs_and_do_not_push() {
    let f = Fixture::create();
    let (_home, s) = service(vec![f.repo.clone()]);
    git(
        &f.repo,
        &[
            "config",
            "--replace-all",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/custom/*",
        ],
    );
    f.advance();
    let head = git(&f.repo, &["rev-parse", "HEAD"]);
    let remote = git(&f.remote, &["rev-parse", "main"]);
    assert!(matches!(
        s.fetch(&f.repo, &Cancel::default()).unwrap(),
        Change::Fetched { .. }
    ));
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        git(&f.repo, &["rev-parse", "refs/remotes/custom/main"]),
        remote
    );
    run(&s, &f, Request::Pull);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), remote);
    assert_eq!(git(&f.remote, &["rev-parse", "main"]), remote);
}

#[test]
fn conflicted_stash_pop_retains_the_stash_and_records_the_actual_repository_state() {
    let f = Fixture::create();
    f.commit("file", "base\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    write(f.repo.join("file"), "stashed\n");
    run(
        &s,
        &f,
        Request::Stash(stash::Request::Save {
            mode: SaveMode::Tracked,
            message: "conflict".into(),
        }),
    );
    let oid = s.stashes(&f.repo, &Cancel::default()).unwrap().entries[0].oid;
    f.commit("file", "other branch contents\n");
    let plan = s
        .prepare_mutation(
            &f.repo,
            Request::Stash(stash::Request::Pop(oid)),
            &Cancel::default(),
        )
        .unwrap();
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(
        s.stashes(&f.repo, &Cancel::default()).unwrap().entries[0].oid,
        oid
    );
    assert!(
        git2::Repository::open(&f.repo)
            .unwrap()
            .index()
            .unwrap()
            .has_conflicts()
    );
    assert!(s.store.repository_report(&f.repo).unwrap().is_some());
}

#[test]
fn safe_switch_preserves_untracked_collisions_and_records_hook_failure_after_checkout() {
    let f = Fixture::create();
    git(&f.repo, &["checkout", "-b", "target"]);
    f.commit("collision", "committed\n");
    git(&f.repo, &["checkout", "main"]);
    write(f.repo.join("collision"), "untracked\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    let plan = s
        .prepare_mutation(
            &f.repo,
            branch(edit::Action::Switch, "target"),
            &Cancel::default(),
        )
        .unwrap();
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(git(&f.repo, &["branch", "--show-current"]), "main");
    assert_eq!(
        fs::read_to_string(f.repo.join("collision")).unwrap(),
        "untracked\n"
    );
    fs::remove_file(f.repo.join("collision")).unwrap();
    let hook = f.repo.join(".git/hooks/post-checkout");
    write(&hook, "#!/bin/sh\necho hook-failed >&2\nexit 1\n");
    fs::set_permissions(hook, fs::Permissions::from_mode(0o700)).unwrap();
    let plan = s
        .prepare_mutation(
            &f.repo,
            branch(edit::Action::Switch, "target"),
            &Cancel::default(),
        )
        .unwrap();
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(git(&f.repo, &["branch", "--show-current"]), "target");
    assert_eq!(
        fs::read_to_string(f.repo.join("collision")).unwrap(),
        "committed\n"
    );
    let history = s.store.operation_history(&f.repo, 10).unwrap();
    assert_eq!(history.len(), 2);
    assert!(matches!(
        history[0].record.operation,
        engine::state::Operation::Workspace {
            completion: engine::state::Completion::Failed { .. },
            ..
        }
    ));
    assert!(s.store.repository_report(&f.repo).unwrap().is_some());
}

#[test]
fn local_upstream_fast_forward_requires_no_remote_and_never_implicitly_sets_tracking() {
    let f = Fixture::create();
    f.commit("local", "incoming\n");
    git(&f.repo, &["branch", "local-upstream"]);
    git(&f.repo, &["reset", "--hard", "HEAD~1"]);
    git(&f.repo, &["remote", "remove", "origin"]);
    git(&f.repo, &["config", "branch.main.remote", "."]);
    git(
        &f.repo,
        &["config", "branch.main.merge", "refs/heads/local-upstream"],
    );
    let (_home, s) = service(vec![f.repo.clone()]);
    run(&s, &f, Request::Pull);
    assert_eq!(
        git(&f.repo, &["rev-parse", "HEAD"]),
        git(&f.repo, &["rev-parse", "local-upstream"])
    );
    assert_eq!(git(&f.repo, &["config", "branch.main.remote"]), ".");
    assert_eq!(
        fs::read_to_string(f.repo.join("local")).unwrap(),
        "incoming\n"
    );
}

#[test]
fn selected_stash_contains_only_reviewed_paths_and_preserves_unselected_staging() {
    let f = Fixture::create();
    for (path, text) in [
        ("chosen.txt", "base\n"),
        ("other.txt", "other base\n"),
        ("old 中文.txt", "rename content\n"),
        ("a[1] 中文.txt", "literal base\n"),
        ("a1 中文.txt", "pattern base\n"),
        (".gitignore", "ignored.txt\n"),
    ] {
        f.commit(path, text);
    }
    write(f.repo.join("other.txt"), "other staged\n");
    write(f.repo.join("unchosen-new.txt"), "unselected staged new\n");
    git(&f.repo, &["add", "--", "other.txt", "unchosen-new.txt"]);
    write(f.repo.join("chosen.txt"), "chosen staged\n");
    git(&f.repo, &["add", "--", "chosen.txt"]);
    write(f.repo.join("chosen.txt"), "chosen staged and unstaged\n");
    git(&f.repo, &["mv", "--", "old 中文.txt", "new 中文.txt"]);
    write(f.repo.join("a[1] 中文.txt"), "selected literal\n");
    write(f.repo.join("a1 中文.txt"), "unselected pattern match\n");
    write(f.repo.join("new \n中文.txt"), "chosen untracked\n");
    write(f.repo.join("untracked-other.txt"), "unselected untracked\n");
    write(f.repo.join("ignored.txt"), "ignored content\n");
    let before_bytes = fs::read(f.repo.join(".git/index")).unwrap();
    let before_head = git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, s) = service(vec![f.repo.clone()]);
    let names = [
        "chosen.txt",
        "old 中文.txt",
        "new 中文.txt",
        "a[1] 中文.txt",
        "new \n中文.txt",
    ];
    let selected = stash::SelectedFiles::new(names.into_iter().map(PathBuf::from)).unwrap();
    let plan = s
        .prepare_mutation(
            &f.repo,
            Request::Stash(stash::Request::Save {
                mode: SaveMode::Selected { files: selected },
                message: "仅勾选文件".into(),
            }),
            &Cancel::default(),
        )
        .unwrap();
    let engine::operations::mutation::Plan::Stash(stash::Plan::Save(review)) = &plan else {
        panic!("save review")
    };
    assert_eq!(review.files.len(), 5);
    assert_eq!(fs::read(f.repo.join(".git/index")).unwrap(), before_bytes);
    s.mutate(&plan, &Cancel::default()).unwrap();
    assert_eq!(
        fs::read_to_string(f.repo.join("chosen.txt")).unwrap(),
        "base\n"
    );
    assert!(f.repo.join("old 中文.txt").exists());
    assert!(!f.repo.join("new 中文.txt").exists());
    assert!(!f.repo.join("new \n中文.txt").exists());
    for (path, text) in [
        ("other.txt", "other staged\n"),
        ("unchosen-new.txt", "unselected staged new\n"),
        ("a1 中文.txt", "unselected pattern match\n"),
        ("untracked-other.txt", "unselected untracked\n"),
        ("ignored.txt", "ignored content\n"),
    ] {
        assert_eq!(fs::read_to_string(f.repo.join(path)).unwrap(), text);
    }
    for (path, text) in [
        ("other.txt", "other staged"),
        ("unchosen-new.txt", "unselected staged new"),
    ] {
        assert_eq!(git(&f.repo, &["show", &format!(":{path}")]), text);
    }
    let stash_paths = git(
        &f.repo,
        &[
            "-c",
            "core.quotePath=false",
            "diff",
            "--no-renames",
            "stash^1",
            "stash",
            "--name-only",
        ],
    );
    assert!(
        !stash_paths.contains("other") && !stash_paths.contains("a1 中文"),
        "{stash_paths}"
    );
    let index_paths = git(&f.repo, &["diff", "stash^1", "stash^2", "--name-only"]);
    assert!(!index_paths.contains("other"), "{index_paths}");
    let untracked = git(
        &f.repo,
        &[
            "-c",
            "core.quotePath=false",
            "ls-tree",
            "-r",
            "--name-only",
            "stash^3",
        ],
    );
    assert!(
        !untracked.contains("other") && !untracked.contains("ignored"),
        "{untracked}"
    );
    assert!(git(&f.repo, &["stash", "list"]).contains("仅勾选文件"));
    let selected_tree = git(&f.repo, &["rev-parse", "stash^2^{tree}"]);
    // 独立演练库中清空其余工作，再验证原生 Git 能完整恢复所选内容和暂存层。
    git(&f.repo, &["reset", "--hard", "HEAD"]);
    git(&f.repo, &["stash", "apply", "--index"]);
    assert_eq!(
        fs::read_to_string(f.repo.join("chosen.txt")).unwrap(),
        "chosen staged and unstaged\n"
    );
    assert!(f.repo.join("new 中文.txt").exists());
    assert!(f.repo.join("new \n中文.txt").exists());
    assert_eq!(git(&f.repo, &["write-tree"]), selected_tree);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), before_head);
    assert!(!f.repo.join(".git/index.lock").exists());
    assert_eq!(s.store.operation_history(&f.repo, 20).unwrap().len(), 1);
}

#[test]
fn stash_default_message_and_tracked_scope_include_added_files_but_exclude_untracked_and_ignored() {
    let f = Fixture::create();
    f.commit("base", "base\n");
    f.commit(".gitignore", "ignored\n");
    write(f.repo.join("added 中文"), "added\n");
    git(&f.repo, &["add", "--", "added 中文"]);
    write(f.repo.join("added 中文"), "added and modified\n");
    write(f.repo.join("untracked"), "new\n");
    write(f.repo.join("ignored"), "ignored\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    run(
        &s,
        &f,
        Request::Stash(stash::Request::Save {
            mode: SaveMode::Tracked,
            message: stash::Message::Automatic,
        }),
    );
    assert!(!f.repo.join("added 中文").exists());
    assert!(f.repo.join("untracked").exists() && f.repo.join("ignored").exists());
    assert!(git(&f.repo, &["stash", "list"]).contains("WIP on main:"));
    let oid = s.stashes(&f.repo, &Cancel::default()).unwrap().entries[0].oid;
    run(&s, &f, Request::Stash(stash::Request::Apply(oid)));
    assert_eq!(
        fs::read_to_string(f.repo.join("added 中文")).unwrap(),
        "added and modified\n"
    );
}

#[test]
fn selected_stash_revalidates_review_and_respects_external_index_locks_and_cancellation() {
    let f = Fixture::create();
    f.commit("chosen", "base\n");
    write(f.repo.join("chosen"), "first change\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    let prepare = || {
        s.prepare_mutation(
            &f.repo,
            Request::Stash(stash::Request::Save {
                mode: SaveMode::Selected {
                    files: stash::SelectedFiles::new([PathBuf::from("chosen")]).unwrap(),
                },
                message: stash::Message::Automatic,
            }),
            &Cancel::default(),
        )
        .unwrap()
    };
    let plan = prepare();
    write(f.repo.join("chosen"), "external change\n");
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    let plan = prepare();
    write(f.repo.join(".git/index.lock"), "external lock");
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(
        fs::read_to_string(f.repo.join(".git/index.lock")).unwrap(),
        "external lock"
    );
    fs::remove_file(f.repo.join(".git/index.lock")).unwrap();
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(s.mutate(&plan, &cancel).is_err());
    assert!(
        s.stashes(&f.repo, &Cancel::default())
            .unwrap()
            .entries
            .is_empty()
    );
    assert_eq!(
        fs::read_to_string(f.repo.join("chosen")).unwrap(),
        "external change\n"
    );
}

#[test]
fn selected_stash_supports_untracked_only_and_preserves_deletions_symlinks_and_executable_modes() {
    use std::os::unix::fs::symlink;
    for scenario in ["untracked", "deleted", "symlink", "executable"] {
        let f = Fixture::create();
        let name = match scenario {
            "untracked" => "-new 中文 [1]",
            "deleted" => {
                f.commit("file", "base\n");
                git(&f.repo, &["rm", "--", "file"]);
                "file"
            }
            "symlink" => {
                f.commit("file", "base\n");
                fs::remove_file(f.repo.join("file")).unwrap();
                symlink("target 中文", f.repo.join("file")).unwrap();
                "file"
            }
            "executable" => {
                f.commit("file", "#!/bin/sh\n");
                fs::set_permissions(f.repo.join("file"), fs::Permissions::from_mode(0o755))
                    .unwrap();
                "file"
            }
            _ => unreachable!(),
        };
        if scenario == "untracked" {
            write(f.repo.join(name), "new file\n");
        }
        let (_home, s) = service(vec![f.repo.clone()]);
        run(
            &s,
            &f,
            Request::Stash(stash::Request::Save {
                mode: SaveMode::Selected {
                    files: stash::SelectedFiles::new([PathBuf::from(name)]).unwrap(),
                },
                message: stash::Message::Automatic,
            }),
        );
        if scenario == "untracked" {
            assert!(!f.repo.join(name).exists());
        } else {
            assert!(f.repo.join(name).is_file());
        }
        let oid = s.stashes(&f.repo, &Cancel::default()).unwrap().entries[0].oid;
        run(&s, &f, Request::Stash(stash::Request::Apply(oid)));
        match scenario {
            "untracked" => assert_eq!(fs::read_to_string(f.repo.join(name)).unwrap(), "new file\n"),
            "deleted" => assert!(!f.repo.join(name).exists()),
            "symlink" => assert_eq!(
                fs::read_link(f.repo.join(name)).unwrap(),
                PathBuf::from("target 中文")
            ),
            "executable" => assert_ne!(
                fs::metadata(f.repo.join(name))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o111,
                0
            ),
            _ => unreachable!(),
        }
    }
}

#[test]
fn selected_stash_in_a_linked_worktree_keeps_the_main_worktree_and_its_index_unchanged() {
    let f = Fixture::create();
    f.commit("file", "base\n");
    let linked = f.temp.path().join("linked 中文");
    git(
        &f.repo,
        &["worktree", "add", "-b", "linked", linked.to_str().unwrap()],
    );
    write(f.repo.join("file"), "main staged\n");
    git(&f.repo, &["add", "--", "file"]);
    let main_index = fs::read(f.repo.join(".git/index")).unwrap();
    write(linked.join("file"), "linked change\n");
    let (_home, s) = service(vec![linked.clone()]);
    let plan = s
        .prepare_mutation(
            &linked,
            Request::Stash(stash::Request::Save {
                mode: SaveMode::Selected {
                    files: stash::SelectedFiles::new([PathBuf::from("file")]).unwrap(),
                },
                message: stash::Message::Automatic,
            }),
            &Cancel::default(),
        )
        .unwrap();
    s.mutate(&plan, &Cancel::default()).unwrap();
    assert_eq!(fs::read_to_string(linked.join("file")).unwrap(), "base\n");
    assert_eq!(
        fs::read_to_string(f.repo.join("file")).unwrap(),
        "main staged\n"
    );
    assert_eq!(fs::read(f.repo.join(".git/index")).unwrap(), main_index);
    let oid = s.stashes(&linked, &Cancel::default()).unwrap().entries[0].oid;
    let restore = s
        .prepare_mutation(
            &linked,
            Request::Stash(stash::Request::Apply(oid)),
            &Cancel::default(),
        )
        .unwrap();
    s.mutate(&restore, &Cancel::default()).unwrap();
    assert_eq!(
        fs::read_to_string(linked.join("file")).unwrap(),
        "linked change\n"
    );
}
