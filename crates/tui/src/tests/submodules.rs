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
    screen.service.configure(ConfigEdit::RepositoryEnvironment {
        path: parent.repo.clone(),
        edit: Edit::Launcher(Some(Launcher {
            program: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                "git=$1; shift; GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null exec \"$git\" -c protocol.file.allow=always \"$@\"".into(),
                "submodule-fixture".into(),
            ],
        })),
    }).unwrap();
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
