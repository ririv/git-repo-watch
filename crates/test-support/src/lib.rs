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
            git_shell().to_owned(),
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
                shell_path(&self.started),
                shell_path(&self.release),
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
/// An absolute fixture path with a drive prefix on Windows.
#[macro_export]
macro_rules! fixture_path {
    ($path:literal) => {
        if cfg!(windows) {
            concat!("C:", $path)
        } else {
            $path
        }
    };
}

/// Use the same Git installation as the application and isolate its configuration.
pub fn git_command() -> Command {
    let mut command = Command::new(if cfg!(windows) {
        "git.exe"
    } else {
        "/usr/bin/git"
    });
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", NULL_DEVICE)
        .env("GIT_TERMINAL_PROMPT", "0");
    command
}

pub const NULL_DEVICE: &str = if cfg!(windows) { "NUL" } else { "/dev/null" };

pub const HOME_VARIABLE: &str = if cfg!(windows) { "USERPROFILE" } else { "HOME" };

/// Return the shell shipped alongside Git for Windows, or the Unix system shell.
pub fn git_shell() -> &'static Path {
    #[cfg(unix)]
    return Path::new("/bin/sh");
    #[cfg(windows)]
    {
        use std::sync::OnceLock;
        static SHELL: OnceLock<PathBuf> = OnceLock::new();
        SHELL.get_or_init(|| {
            let output = git_command()
                .arg("--exec-path")
                .output()
                .expect("Git for Windows");
            assert!(output.status.success());
            let path = String::from_utf8(output.stdout).expect("Git for Windows exec path");
            let installation = Path::new(path.trim())
                .ancestors()
                .nth(3)
                .expect("Git for Windows installation");
            let shell = installation.join("usr/bin/sh.exe");
            assert!(
                shell.is_file(),
                "Git for Windows shell: {}",
                shell.display()
            );
            shell
        })
    }
}

pub fn shell_path(path: &Path) -> String {
    let value = path.to_str().expect("fixture path");
    #[cfg(windows)]
    return value
        .strip_prefix(r"\\?\")
        .unwrap_or(value)
        .replace('\\', "/");
    #[cfg(unix)]
    return value.into();
}

/// Write a Git hook in the format expected by the local Git installation.
pub fn write_git_hook(path: impl AsRef<Path>, script: &str) {
    let path = path.as_ref();
    fs::write(path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
}

pub fn git(path: &Path, args: &[&str]) -> String {
    let out = git_command()
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .trim_end_matches(['\r', '\n'])
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
        ("core.autocrlf", "false"),
    ] {
        git(path, &["config", key, value]);
    }
    let excludes = path.join(".git/fixture-excludes");
    fs::write(&excludes, []).unwrap();
    git(
        path,
        &["config", "core.excludesFile", &shell_path(&excludes)],
    );
    git(
        path,
        &[
            "config",
            "core.hooksPath",
            &shell_path(&path.join(".git/hooks")),
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
        git(&repo, &["remote", "add", "origin", &shell_path(&remote)]);
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
            &["clone", &shell_path(&self.remote), &shell_path(&peer)],
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
