use engine::{
    cancel::Cancel,
    local::{
        files::{
            self, Area,
            patch::{Document, Source},
        },
        history::{self, Parent},
    },
};
use std::{fs, path::Path};
use test_support::{Fixture, git};
fn bytes(version: u8) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0snapshot".to_vec();
    bytes.push(version);
    bytes
}
fn document(f: &Fixture, name: &str, area: Area) -> Document {
    let snapshot = files::read(&f.repo).unwrap();
    let change = snapshot
        .changes
        .iter()
        .find(|change| change.location.name() == Path::new(name) && change.area == area)
        .unwrap();
    files::document(&snapshot, change, &Cancel::default()).unwrap()
}
fn sources(document: &Document) -> (&Source, &Source) {
    let Document::Patch { files, .. } = document else {
        panic!("image patch expected")
    };
    assert_eq!(files.len(), 1);
    (&files[0].before, &files[0].after)
}
fn version(source: &Source) -> u8 {
    let Source::Image(image) = source else {
        panic!("image source expected")
    };
    *image.bytes.last().unwrap()
}
#[test]
fn image_snapshots_keep_head_index_and_worktree_versions_separate() {
    let f = Fixture::create();
    let path = f.repo.join("中文 image.png");
    fs::write(&path, bytes(1)).unwrap();
    git(&f.repo, &["add", "."]);
    git(&f.repo, &["commit", "-m", "first image"]);
    fs::write(&path, bytes(2)).unwrap();
    git(&f.repo, &["add", "."]);
    fs::write(&path, bytes(3)).unwrap();
    let index = fs::read(f.repo.join(".git/index")).unwrap();
    let refs = git(&f.repo, &["show-ref"]);
    let staged = document(&f, "中文 image.png", Area::Staged);
    assert_eq!(
        (version(sources(&staged).0), version(sources(&staged).1)),
        (1, 2)
    );
    let working = document(&f, "中文 image.png", Area::Unstaged);
    assert_eq!(
        (version(sources(&working).0), version(sources(&working).1)),
        (2, 3)
    );
    fs::write(f.repo.join("new-image"), bytes(4)).unwrap();
    let new = document(&f, "new-image", Area::Untracked);
    assert!(matches!(sources(&new).0, Source::Absent));
    assert_eq!(version(sources(&new).1), 4);
    fs::remove_file(f.repo.join("new-image")).unwrap();
    fs::remove_file(&path).unwrap();
    let deleted = document(&f, "中文 image.png", Area::Unstaged);
    assert_eq!(version(sources(&deleted).0), 2);
    assert!(matches!(sources(&deleted).1, Source::Absent));
    assert_eq!(fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(git(&f.repo, &["show-ref"]), refs);
}
#[test]
fn historical_image_sources_are_git_objects_even_when_index_and_worktree_differ() {
    let f = Fixture::create();
    let path = f.repo.join("image.webp");
    fs::write(&path, bytes(1)).unwrap();
    git(&f.repo, &["add", "."]);
    git(&f.repo, &["commit", "-m", "first image"]);
    fs::write(&path, bytes(2)).unwrap();
    git(&f.repo, &["add", "."]);
    git(&f.repo, &["commit", "-m", "second image"]);
    let oid = git(&f.repo, &["rev-parse", "HEAD"]).parse().unwrap();
    fs::write(&path, bytes(3)).unwrap();
    git(&f.repo, &["add", "."]);
    fs::write(&path, bytes(4)).unwrap();
    let detail = history::detail(&f.repo, oid, Parent::First, &Cancel::default()).unwrap();
    let document = history::document(&detail, &detail.changes[0], &Cancel::default()).unwrap();
    assert_eq!(
        (version(sources(&document).0), version(sources(&document).1)),
        (1, 2)
    );
}
#[test]
fn encoded_images_have_an_independent_size_limit_and_symlinks_are_not_followed() {
    use std::{io::Write, os::unix::fs::symlink};
    let f = Fixture::create();
    let path = f.repo.join("large.png");
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(&bytes(1)).unwrap();
    file.set_len(1024 * 1024 + 1).unwrap();
    assert!(matches!(
        sources(&document(&f, "large.png", Area::Untracked)).1,
        Source::Image(_)
    ));
    file.set_len(files::media::MAX_IMAGE_BYTES as u64 + 1)
        .unwrap();
    let snapshot = files::read(&f.repo).unwrap();
    let change = snapshot
        .changes
        .iter()
        .find(|change| change.location.name() == Path::new("large.png"))
        .unwrap();
    assert!(files::document(&snapshot, change, &Cancel::default()).is_err());
    fs::remove_file(path).unwrap();
    let outside = f.temp.path().join("outside.png");
    fs::write(&outside, bytes(2)).unwrap();
    symlink(outside, f.repo.join("link.png")).unwrap();
    assert!(matches!(
        document(&f, "link.png", Area::Untracked),
        Document::Notice(_)
    ));
}
