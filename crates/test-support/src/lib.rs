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
#[derive(Clone, Copy)]
pub enum GitOperation {
    Fetch,
    Commit,
}

/// Hold an actual Git operation before execution until the test releases it.
pub struct GitGate {
    operation: GitOperation,
    pub started: PathBuf,
    release: PathBuf,
}
impl GitGate {
    pub fn new(fixture: &Fixture, operation: GitOperation) -> Self {
        let root = fixture.repo.parent().unwrap();
        Self {
            operation,
            started: root.join("git.started"),
            release: root.join("git.release"),
        }
    }

    pub fn launcher(&self) -> (PathBuf, Vec<String>) {
        let operation = match self.operation {
            GitOperation::Fetch => "fetch",
            GitOperation::Commit => "commit",
        };
        (
            "/bin/sh".into(),
            vec![
                "-c".into(),
                r#"
operation=$1
started=$2
release=$3
shift 3
if [ "$2" = "$operation" ]; then
    : > "$started" || exit 1
    while [ ! -e "$release" ]; do sleep 0.01; done
fi
exec "$@"
"#
                .into(),
                "git-gate".into(),
                operation.into(),
                self.started.to_str().unwrap().into(),
                self.release.to_str().unwrap().into(),
            ],
        )
    }

    pub fn release(&self) {
        fs::write(&self.release, b"release").unwrap();
    }
}
impl Drop for GitGate {
    fn drop(&mut self) {
        let _ = fs::write(&self.release, b"release");
    }
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
