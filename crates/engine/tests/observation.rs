use engine::{
    cancel::Cancel,
    domain::PushPolicy,
    observation::{HistoryWindow, Scope, watch::Change},
    scheduler::Manager,
    service::Service,
    state::{Config, Store},
};
use test_support::{Fixture, git, write};
fn service(path: std::path::PathBuf) -> (tempfile::TempDir, Service) {
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().into());
    let config = Config {
        repositories: vec![path],
        scheduled_actions: engine::workflow::Plan::from_policy(PushPolicy::Disabled),
        ..Config::default()
    };
    store.save_config(&config).unwrap();
    let manager = Manager::for_user(home.path().into(), unsafe { libc::getuid() }).unwrap();
    (home, Service::new(store, manager))
}
#[test]
fn local_observation_never_fetches_writes_git_or_persists_check_history() {
    let f = Fixture::create();
    f.commit("文件.ts", "export const value = 1;\n");
    f.advance();
    write(f.repo.join("文件.ts"), "export const value = 2;\n");
    git(&f.repo, &["add", "文件.ts"]);
    write(f.repo.join("文件.ts"), "export const value = 3;\n");
    let (_home, service) = service(f.repo.clone());
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let refs = git(&f.repo, &["show-ref"]);
    let before = git(&f.repo, &["diff", "HEAD"]);
    let snapshot = service
        .observe(
            &f.repo,
            &Change::all(),
            Scope::Visible {
                history: HistoryWindow {
                    count: 100,
                    anchor: None,
                },
                include_ignored: false,
            },
            &Cancel::default(),
        )
        .unwrap()
        .unwrap();
    let detail = snapshot.detail.unwrap();
    assert_eq!(detail.files.unwrap().changes.len(), 2);
    assert!(detail.overview.is_ok());
    assert!(detail.branches.is_ok());
    assert!(detail.stashes.is_ok());
    assert_eq!(detail.history.unwrap()[0].entries.len(), 2);
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(git(&f.repo, &["show-ref"]), refs);
    assert_eq!(git(&f.repo, &["diff", "HEAD"]), before);
    assert!(service.store.report().unwrap().is_none());
    assert!(service.store.repository_reports().unwrap().is_empty());
}
#[test]
fn history_window_keeps_selected_commit_when_new_commits_cross_the_page_boundary() {
    let f = Fixture::create();
    for index in 0..105 {
        git(
            &f.repo,
            &["commit", "--allow-empty", "-m", &format!("commit {index}")],
        );
    }
    let (_home, service) = service(f.repo.clone());
    let first = service
        .history(
            &f.repo,
            engine::local::history::Request::Start,
            &Cancel::default(),
        )
        .unwrap();
    let anchor = first.entries.last().unwrap().oid;
    git(
        &f.repo,
        &["commit", "--allow-empty", "-m", "external new commit"],
    );
    let snapshot = service
        .observe(
            &f.repo,
            &Change::all(),
            Scope::Visible {
                history: HistoryWindow {
                    count: 100,
                    anchor: Some(anchor),
                },
                include_ignored: false,
            },
            &Cancel::default(),
        )
        .unwrap()
        .unwrap();
    let pages = snapshot.detail.unwrap().history.unwrap();
    assert_eq!(pages.len(), 2);
    assert!(
        pages
            .iter()
            .flat_map(|page| &page.entries)
            .any(|entry| entry.oid == anchor)
    );
    assert_eq!(pages[0].entries[0].subject, "external new commit");
}
