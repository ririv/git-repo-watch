use engine::{
    cancel::Cancel,
    local::remotes::{self, Direction},
};
use test_support::{Fixture, git};

#[test]
fn reads_all_configured_endpoints_and_distinct_push_addresses_without_mutating_git() {
    let f = Fixture::create();
    git(
        &f.repo,
        &[
            "remote",
            "set-url",
            "origin",
            "https://example.com/team/actual-project.git",
        ],
    );
    git(
        &f.repo,
        &[
            "config",
            "--add",
            "remote.origin.url",
            "https://mirror.example.com/team/actual-project.git",
        ],
    );
    git(
        &f.repo,
        &[
            "config",
            "--add",
            "remote.origin.pushurl",
            "https://example.com/team/actual-project.git",
        ],
    );
    git(
        &f.repo,
        &[
            "config",
            "--add",
            "remote.origin.pushurl",
            "https://example.com/fork/project.git",
        ],
    );
    git(
        &f.repo,
        &[
            "remote",
            "add",
            "upstream",
            "git@example.com:team/original.git",
        ],
    );
    let config = std::fs::read(f.repo.join(".git/config")).unwrap();
    let head = git(&f.repo, &["rev-parse", "HEAD"]);
    let endpoints = remotes::read(&f.repo, &Cancel::default()).unwrap();
    assert_eq!(endpoints.len(), 4);
    assert_eq!(endpoints[0].name, "origin");
    assert_eq!(
        endpoints[0].url,
        "https://example.com/team/actual-project.git"
    );
    assert_eq!(
        endpoints[1].url,
        "https://mirror.example.com/team/actual-project.git"
    );
    assert_eq!(endpoints[2].direction, Direction::Push);
    assert_eq!(endpoints[2].url, "https://example.com/fork/project.git");
    assert_eq!(endpoints[3].name, "upstream");
    assert_eq!(endpoints[3].url, "git@example.com:team/original.git");
    assert_eq!(std::fs::read(f.repo.join(".git/config")).unwrap(), config);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), head);
}

#[test]
fn public_endpoints_do_not_expose_embedded_credentials_or_query_tokens() {
    let f = Fixture::create();
    git(
        &f.repo,
        &[
            "remote",
            "set-url",
            "origin",
            "https://user:secret-token@example.com/team/name.git?token=private#fragment",
        ],
    );
    let endpoints = remotes::read(&f.repo, &Cancel::default()).unwrap();
    assert_eq!(endpoints[0].url, "https://example.com/team/name.git");
    git(
        &f.repo,
        &[
            "remote",
            "set-url",
            "origin",
            "ssh://git@example.com:2222/team/name.git",
        ],
    );
    assert_eq!(
        remotes::read(&f.repo, &Cancel::default()).unwrap()[0].url,
        "ssh://git@example.com:2222/team/name.git"
    );
}

#[test]
fn local_repositories_and_local_remote_paths_are_distinct() {
    let f = Fixture::create();
    let endpoints = remotes::read(&f.repo, &Cancel::default()).unwrap();
    assert_eq!(endpoints[0].url, f.remote.to_string_lossy());
    git(&f.repo, &["remote", "remove", "origin"]);
    assert!(
        remotes::read(&f.repo, &Cancel::default())
            .unwrap()
            .is_empty()
    );
}
