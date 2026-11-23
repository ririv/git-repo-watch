use super::*;
use engine::{
    git_environment::{Edit, Launcher},
    local::submodules::State,
    operations::mutation::Plan,
    service::ConfigEdit,
};
use std::path::Path;

#[test]
fn adds_submodule_after_confirmation_and_reads_nested_tree() {
    let parent = support::Fixture::create();
    let child = support::Fixture::create();
    let source = parent.temp.path().join("共享 模块");
    support::init(&source);
    support::git(
        &source,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "--",
            child.remote.to_str().unwrap(),
            "deps/内层",
        ],
    );
    support::git(&source, &["commit", "-m", "Add nested submodule"]);
    let original_head = support::git(&parent.repo, &["rev-parse", "HEAD"]);

    let (_home, mut screen) = fixture(vec![parent.repo.clone()]);
    allow_local_git(&screen, &parent.repo);
    screen.open_repository(parent.repo.clone());
    command(&mut screen, "repo submodule");
    wait(&mut screen);
    assert!(matches!(&screen.mode, Mode::Submodules(tree) if tree.entries.is_empty()));
    let empty = render(&mut screen, 160, 26);
    let first_row = empty
        .lines()
        .skip_while(|line| !line.starts_with('┌') || !line.contains(i18n::tr!(SubmodulesTitle)))
        .nth(1)
        .expect("submodule panel body");
    assert_eq!(first_row.trim_matches(['│', ' ']), i18n::tr!(NoSubmodules));
    let path = "modules/外层 库";
    command(
        &mut screen,
        &format!("repo submodule add \"{}\" \"{path}\"", source.display()),
    );
    wait(&mut screen);
    assert!(
        matches!(
            &screen.mode,
            Mode::Confirm {
                operation: Confirmation::Mutation { plan, .. },
                choice: crate::confirmation::Choice::Return,
            } if matches!(plan.as_ref(), Plan::AddSubmodule(_))
        ),
        "{:?}",
        screen.message
    );
    let review = render(&mut screen, 180, 32);
    assert!(review.contains(path) && review.contains(".gitmodules"));
    assert!(!parent.repo.join(path).exists());
    assert!(!parent.repo.join(".gitmodules").exists());

    key(&mut screen, KeyCode::Left);
    key(&mut screen, KeyCode::Enter);
    wait(&mut screen);
    assert_eq!(
        support::git(
            &parent.repo,
            &[
                "diff",
                "--cached",
                "--name-only",
                "--no-renames",
                "--",
                ".gitmodules"
            ]
        ),
        ".gitmodules"
    );
    assert!(support::git(&parent.repo, &["ls-files", "-s", "--", path]).starts_with("160000 "));
    assert_eq!(
        support::git(&parent.repo, &["rev-parse", "HEAD"]),
        original_head
    );
    assert!(
        matches!(
            &screen.mode,
            Mode::Submodules(tree) if matches!(
                &tree.entries[..],
                [entry] if entry.path == Path::new(path) && matches!(
                    &entry.state,
                    State::Initialized { children } if matches!(
                        &children[..],
                        [child] if child.path == Path::new("deps/内层") && matches!(child.state, State::Uninitialized)
                    )
                )
            )
        ),
        "{:?}",
        screen.message
    );
    let tree = render(&mut screen, 160, 26);
    assert!(tree.contains("└─ modules/外层 库"));
    assert!(tree.contains("   └─ deps/内层"));
    assert!(tree.contains(i18n::tr!(SubmoduleUninitialized)));

    support::git(
        &parent.repo.join(path),
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "update",
            "--init",
        ],
    );
    command(&mut screen, "refresh");
    wait(&mut screen);
    let tree = render(&mut screen, 160, 26);
    assert!(tree.contains("   └─ deps/内层"));
    assert!(!tree.contains(i18n::tr!(SubmoduleUninitialized)));
    screen.shutdown();
}

#[test]
fn adding_submodule_revalidates_the_destination_after_preview() {
    let parent = support::Fixture::create();
    let (_home, mut screen) = fixture(vec![parent.repo.clone()]);
    screen.open_repository(parent.repo.clone());
    command(
        &mut screen,
        &format!(
            "repo submodule add \"{}\" deps/module",
            parent.remote.display()
        ),
    );
    wait(&mut screen);
    assert!(
        matches!(&screen.mode, Mode::Confirm { .. }),
        "{:?}",
        screen.message
    );

    let destination = parent.repo.join("deps/module");
    std::fs::create_dir_all(&destination).unwrap();
    support::write(destination.join("keep"), "user file");
    key(&mut screen, KeyCode::Left);
    key(&mut screen, KeyCode::Enter);
    wait(&mut screen);
    assert_eq!(
        std::fs::read_to_string(destination.join("keep")).unwrap(),
        "user file"
    );
    assert!(!parent.repo.join(".gitmodules").exists());
    assert!(support::git(&parent.repo, &["diff", "--cached", "--name-only"]).is_empty());
    screen.shutdown();
}

fn allow_local_git(screen: &Screen, path: &Path) {
    screen.service.configure(ConfigEdit::RepositoryEnvironment {
        path: path.to_owned(),
        edit: Edit::Launcher(Some(Launcher {
            program: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                "git=$1; shift; GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null exec \"$git\" -c protocol.file.allow=always \"$@\"".into(),
                "submodule-fixture".into(),
            ],
        })),
    }).unwrap();
}

#[test]
fn initializes_nested_submodules_and_updates_a_selected_checkout_to_the_index() {
    use engine::operations::submodule::checkout::Mode as Checkout;
    let nested = support::Fixture::create();
    let module = support::Fixture::create();
    module.commit("tracked", "old\n");
    support::git(
        &module.repo,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            nested.remote.to_str().unwrap(),
            "deps/nested",
        ],
    );
    support::git(&module.repo, &["commit", "-m", "Add nested submodule"]);
    support::git(&module.repo, &["push"]);
    let parent = support::Fixture::create();
    let path = "modules/外层 库";
    support::git(
        &parent.repo,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            module.remote.to_str().unwrap(),
            path,
        ],
    );
    support::git(&parent.repo, &["commit", "-m", "Record submodule"]);
    let repo = parent.temp.path().join("checkout");
    support::git(
        parent.temp.path(),
        &[
            "clone",
            "--no-recurse-submodules",
            parent.repo.to_str().unwrap(),
            repo.to_str().unwrap(),
        ],
    );
    let repo = repo.canonicalize().unwrap();
    let head = support::git(&repo, &["rev-parse", "HEAD"]);
    let old = support::git(&module.repo, &["rev-parse", "HEAD"]);
    let checkout = repo.join(path);
    let (_home, mut screen) = fixture(vec![repo.clone()]);
    allow_local_git(&screen, &repo);
    screen.open_repository(repo.clone());
    command(&mut screen, "repo submodule");
    wait(&mut screen);
    assert!(render(&mut screen, 160, 26).contains(i18n::tr!(SubmoduleUninitialized)));

    command(&mut screen, "repo submodule init");
    wait(&mut screen);
    assert_checkout_preview(&screen, Checkout::Initialize);
    assert!(!checkout.join(".git").exists());
    let review = render(&mut screen, 180, 26);
    assert!(review.contains(path) && review.contains(&old[..12]));
    key(&mut screen, KeyCode::Left);
    key(&mut screen, KeyCode::Enter);
    wait(&mut screen);
    if let Mode::Error { detail, .. } = &screen.mode {
        panic!("{detail}");
    }
    assert!(checkout.join(".git").is_file(), "{:?}", screen.message);
    assert_eq!(support::git(&checkout, &["rev-parse", "HEAD"]), old);
    assert!(checkout.join("deps/nested/.git").is_file());
    let tree = render(&mut screen, 160, 26);
    assert!(tree.contains("   └─ deps/nested"));
    assert!(!tree.contains(i18n::tr!(SubmoduleUninitialized)));

    module.commit("tracked", "new\n");
    support::git(&module.repo, &["push"]);
    let target = support::git(&module.repo, &["rev-parse", "HEAD"]);
    let indexed = format!("160000,{target},{path}");
    support::git(&repo, &["update-index", "--cacheinfo", &indexed]);
    let staged = support::git(&repo, &["ls-files", "--stage"]);
    let update = format!("repo submodule update \"{path}\"");
    command(&mut screen, &update);
    wait(&mut screen);
    assert_checkout_preview(&screen, Checkout::Update);
    let review = render(&mut screen, 180, 26);
    assert!(review.contains(&old[..12]) && review.contains(&target[..12]));

    support::write(checkout.join("tracked"), "local edit\n");
    key(&mut screen, KeyCode::Left);
    key(&mut screen, KeyCode::Enter);
    wait(&mut screen);
    assert_eq!(
        std::fs::read_to_string(checkout.join("tracked")).unwrap(),
        "local edit\n"
    );
    assert_eq!(support::git(&checkout, &["rev-parse", "HEAD"]), old);

    support::git(&checkout, &["restore", "--", "tracked"]);
    if matches!(screen.mode, Mode::Error { .. }) {
        key(&mut screen, KeyCode::Esc);
    }
    command(&mut screen, &update);
    wait(&mut screen);
    assert_checkout_preview(&screen, Checkout::Update);
    key(&mut screen, KeyCode::Left);
    key(&mut screen, KeyCode::Enter);
    wait(&mut screen);
    assert_eq!(support::git(&checkout, &["rev-parse", "HEAD"]), target);
    assert_eq!(support::git(&repo, &["ls-files", "--stage"]), staged);
    assert_eq!(support::git(&repo, &["rev-parse", "HEAD"]), head);
    screen.shutdown();
}

fn assert_checkout_preview(
    screen: &Screen,
    checkout: engine::operations::submodule::checkout::Mode,
) {
    assert!(
        matches!(
            &screen.mode,
            Mode::Confirm {
                operation: Confirmation::Mutation { plan, .. },
                choice: crate::confirmation::Choice::Return,
            } if matches!(plan.as_ref(), Plan::CheckoutSubmodules(plan) if plan.mode() == checkout)
        ),
        "{:?}",
        screen.message
    );
}
