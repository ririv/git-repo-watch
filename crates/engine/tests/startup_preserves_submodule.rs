use engine::{
    cancel::Cancel,
    local::history::Request,
    scheduler::Manager,
    service::Service,
    state::{Config, Store},
};
use std::fs;
use test_support::{Fixture, git, write};

#[test]
fn checks_and_repository_readers_preserve_uncommitted_submodule_migration() {
    let parent = Fixture::create();
    let source = Fixture::create();
    fs::create_dir_all(source.repo.join("packages/cpython")).unwrap();
    source.commit(
        "packages/cpython/recipe.py",
        "def install():\n    publish_install()\n",
    );
    git(&source.repo, &["push", "origin", "main"]);
    git(
        &parent.repo,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            &test_support::shell_path(&source.remote),
            "recipes",
        ],
    );
    git(&parent.repo, &["commit", "-am", "add recipes submodule"]);
    let child = parent.repo.join("recipes");
    let recipe = child.join("packages/cpython/recipe.py");
    let old_edit = "from project.build.python import write_details\ndef install():\n    publish_install()\n    write_details()\n";
    write(&recipe, old_edit);
    let migrated = "def install():\n    phase.run(publish_install)\n";
    write(&recipe, migrated);
    let adapter = parent.repo.join("toolchain/python/details.py");
    fs::create_dir_all(adapter.parent().unwrap()).unwrap();
    write(
        &adapter,
        "def write_details():\n    return 'consumer output'\n",
    );
    let frame = parent.repo.join("frame.py");
    write(
        &frame,
        "from toolchain.python.details import write_details\n",
    );

    // 远程仍保留旧树，并产生真实更新；fetch 必须只更新远程跟踪引用。
    source.commit("remote-note", "new remote commit\n");
    git(&source.repo, &["push", "origin", "main"]);
    parent.advance();
    let paths = vec![parent.repo.clone(), child.clone()];
    let heads = paths
        .iter()
        .map(|path| git(path, &["rev-parse", "HEAD"]))
        .collect::<Vec<_>>();
    let index_paths = paths
        .iter()
        .map(|path| {
            git2::Repository::open(path)
                .unwrap()
                .index()
                .unwrap()
                .path()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    let observed = [
        recipe.clone(),
        adapter,
        frame,
        index_paths[0].clone(),
        index_paths[1].clone(),
    ];
    let before = observed
        .iter()
        .map(|path| {
            (
                fs::read(path).unwrap(),
                fs::metadata(path).unwrap().modified().unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().into());
    store
        .save_config(&Config {
            repositories: paths.clone(),
            max_concurrency: 8,
            fetch_on_startup: false,
            auto_fetch: Default::default(),
            experimental_features: true,
            ..Config::default()
        })
        .unwrap();
    let manager = Manager::for_current_user_in(home.path().into()).unwrap();
    let service = Service::new(store, manager);
    let cancel = Cancel::default();
    let assert_scope = |report: engine::domain::Report| {
        assert_eq!(report.repositories.len(), 2);
    };
    assert_scope(service.run(&cancel, |_| {}).unwrap());
    #[cfg(feature = "developer-scheduler")]
    assert_scope(service.run_scheduled(&cancel, |_| {}).unwrap());
    for (i, path) in paths.iter().enumerate() {
        service.overview(path, &Cancel::default()).unwrap();
        service.branches(path, &Cancel::default()).unwrap();
        service
            .history(path, Request::Start, &Cancel::default())
            .unwrap();
        service.stashes(path, &Cancel::default()).unwrap();
        let files = service.files(path, &Cancel::default()).unwrap();
        for change in &files.changes {
            service.diff(&files, change, &Cancel::default()).unwrap();
        }
        assert_eq!(git(path, &["rev-parse", "HEAD"]), heads[i]);
    }
    assert_eq!(
        git(&child, &["rev-parse", "origin/main"]),
        git(&source.repo, &["rev-parse", "HEAD"])
    );
    for (path, (bytes, modified)) in observed.iter().zip(before) {
        assert_eq!(fs::read(path).unwrap(), bytes, "{}", path.display());
        assert_eq!(
            fs::metadata(path).unwrap().modified().unwrap(),
            modified,
            "{}",
            path.display()
        );
    }
    assert_eq!(fs::read_to_string(recipe).unwrap(), migrated);
}
