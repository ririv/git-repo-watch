//! Shared isolated Git fixtures for workspace tests; never a runtime dependency.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;
pub struct Fixture {
    pub temp: TempDir,
    pub repo: PathBuf,
    pub remote: PathBuf,
}
pub fn git(path: &Path, args: &[&str]) -> String {
    let out = Command::new("/usr/bin/git")
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .trim_end_matches('\n')
        .to_owned()
}
pub fn write(path: impl AsRef<Path>, text: &str) {
    fs::write(path, text).unwrap()
}
pub fn init(path: &Path) {
    fs::create_dir_all(path).unwrap();
    git(path, &["init", "-b", "main"]);
    for (key, value) in [
        ("user.name", "测试"),
        ("user.email", "test@example.invalid"),
        ("commit.gpgsign", "false"),
        ("core.excludesFile", "/dev/null"),
    ] {
        git(path, &["config", key, value]);
    }
    git(
        path,
        &[
            "config",
            "core.hooksPath",
            path.join(".git/hooks").to_str().unwrap(),
        ],
    );
}
impl Fixture {
    pub fn create() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let repo = root.join("repo");
        let remote = root.join("remote.git");
        init(&repo);
        git(&repo, &["commit", "--allow-empty", "-m", "initial"]);
        fs::create_dir(&remote).unwrap();
        git(&remote, &["init", "--bare", "-b", "main"]);
        git(
            &repo,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        git(&repo, &["push", "-u", "origin", "main"]);
        Self { temp, repo, remote }
    }
    pub fn commit(&self, file: &str, content: &str) {
        write(self.repo.join(file), content);
        git(&self.repo, &["add", "--", file]);
        git(&self.repo, &["commit", "-m", file]);
    }
    pub fn advance(&self) {
        let peer = self.temp.path().join("peer");
        git(
            self.temp.path(),
            &[
                "clone",
                self.remote.to_str().unwrap(),
                peer.to_str().unwrap(),
            ],
        );
        git(
            &peer,
            &[
                "-c",
                "user.name=测试",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--allow-empty",
                "-m",
                "remote ahead",
            ],
        );
        git(&peer, &["push"]);
    }
}
