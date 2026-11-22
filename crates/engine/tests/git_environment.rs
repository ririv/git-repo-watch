use engine::{
    cancel::Cancel,
    domain::Publication,
    git_environment::{Edit, Launcher},
    scheduler::Manager,
    service::{ConfigEdit, Service},
    state::{Config, Store},
};
use std::{fs, os::unix::fs::PermissionsExt, path::Path};
use test_support::{Fixture, write};

fn executable(path: &Path, body: &str) {
    write(path, body);
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

#[test]
fn manual_and_scheduled_pushes_apply_only_their_worktree_git_environment() {
    let first = Fixture::create();
    let second = Fixture::create();
    first.commit("first", "first");
    second.commit("second", "second");
    let tools = first.repo.join("tools with space");
    fs::create_dir(&tools).unwrap();
    executable(
        &tools.join("gw-fixture-hook-tool"),
        "#!/bin/sh\n[ \"$GW_FIXTURE_WORKTREE\" = 'value with space' ]\n",
    );
    executable(
        &tools.join("gw-fixture-launcher"),
        "#!/bin/sh\n[ \"$1\" = 'literal argument' ] || exit 8\nshift\nexec \"$@\"\n",
    );
    executable(
        &first.repo.join(".git/hooks/pre-push"),
        "#!/bin/sh\nexec gw-fixture-hook-tool\n",
    );
    executable(
        &second.repo.join(".git/hooks/pre-push"),
        "#!/bin/sh\n[ -z \"$GW_FIXTURE_WORKTREE\" ] && ! command -v gw-fixture-hook-tool >/dev/null\n",
    );
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().to_owned());
    store
        .save_config(&Config {
            repositories: vec![first.repo.clone(), second.repo.clone()],
            #[cfg(feature = "developer-scheduler")]
            scheduled_actions: "fetch push inspect".parse().unwrap(),
            ..Config::default()
        })
        .unwrap();
    let service = Service::new(
        store,
        Manager::for_user(home.path().to_owned(), unsafe { libc::getuid() }).unwrap(),
    );
    let configure = |edit| {
        service
            .configure(ConfigEdit::RepositoryEnvironment {
                path: first.repo.clone(),
                edit,
            })
            .unwrap()
    };
    configure(Edit::Path(vec!["tools with space".into()]));
    configure(Edit::Set {
        name: "GW_FIXTURE_WORKTREE".into(),
        value: "value with space".into(),
    });
    configure(Edit::Launcher(Some(Launcher {
        program: "gw-fixture-launcher".into(),
        args: vec!["literal argument".into()],
    })));
    let cancel = Cancel::default();
    let plan = service.prepare_push(&first.repo, &cancel).unwrap();
    assert!(matches!(
        service.push(&plan, &cancel).unwrap(),
        Publication::Pushed { .. }
    ));
    #[cfg(feature = "developer-scheduler")]
    {
        first.commit("scheduled", "scheduled");
        let report = service.run_scheduled(&cancel, |_| {}).unwrap();
        assert_eq!(report.repositories.len(), 2);
        for repository in report.repositories {
            assert!(
                matches!(repository.pushes(), [Publication::Pushed { .. }]),
                "{repository:?}"
            );
        }
    }
    configure(Edit::Unset("GW_FIXTURE_WORKTREE".into()));
    configure(Edit::Launcher(None));
    configure(Edit::Path(vec![]));
    assert!(
        service
            .store
            .config()
            .unwrap()
            .repository_environment
            .is_empty()
    );
}
