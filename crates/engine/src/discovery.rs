//! Git-ignore-aware discovery without walking ignored directory trees.
mod request;
use crate::{cancel::Cancel, local, state};
use anyhow::{Context, Result, ensure};
pub use request::Request;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Record {
    Scanned { path: PathBuf },
    Failed { path: PathBuf, error: String },
}
impl Record {
    pub fn path(&self) -> &Path {
        match self {
            Self::Scanned { path } | Self::Failed { path, .. } => path,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub root: PathBuf,
    pub scanned_at: String,
    pub excluded_roots: Vec<PathBuf>,
    pub repositories: Vec<Record>,
    pub non_repository_markers: Vec<PathBuf>,
}
#[derive(Debug, Default, Clone)]
struct Tree(BTreeMap<std::ffi::OsString, Tree>);
fn include(tree: &mut Tree, path: &Path, directory: bool) {
    let mut parts = path
        .components()
        .map(|c| c.as_os_str().to_owned())
        .collect::<Vec<_>>();
    if !directory {
        parts.pop();
    }
    let mut current = tree;
    for part in parts {
        current = current.0.entry(part).or_default();
    }
}
struct RepositoryScope {
    repo: git2::Repository,
    worktree: PathBuf,
    tracked: Tree,
}
impl RepositoryScope {
    fn open(root: &Path) -> Result<Self> {
        let repo = local::open(root)?;
        let mut tracked = Tree::default();
        for entry in repo.index()?.iter() {
            let path = Path::new(
                std::str::from_utf8(&entry.path)
                    .context(i18n::tr!(ScanEncounteredANonUtf8Tracked))?,
            );
            include(&mut tracked, path, entry.mode == 0o160000);
        }
        let worktree =
            fs::canonicalize(repo.workdir().context(i18n::tr!(RepositoryHasNoWorktree))?)?;
        Ok(Self {
            repo,
            worktree,
            tracked,
        })
    }
    fn allows(&self, dir: &Path) -> Result<bool> {
        let relative = dir.strip_prefix(&self.worktree)?;
        let mut tracked = &self.tracked;
        let mut included = true;
        for part in relative.components() {
            match tracked.0.get(part.as_os_str()) {
                Some(next) => tracked = next,
                None => {
                    included = false;
                    break;
                }
            }
        }
        Ok(included || !self.repo.is_path_ignored(relative)?)
    }
}
#[derive(PartialEq, Eq)]
enum Marker {
    Absent,
    Repository,
    OtherFile,
}
fn marker(dir: &Path) -> Result<Marker> {
    let path = dir.join(".git");
    match fs::symlink_metadata(&path) {
        Ok(info) if info.is_dir() => Ok(Marker::Repository),
        Ok(info) if info.is_file() => {
            use std::io::Read;
            let mut f = fs::File::open(path)?;
            let mut buf = [0; 8];
            let n = f.read(&mut buf)?;
            Ok(if &buf[..n] == b"gitdir: " {
                Marker::Repository
            } else {
                Marker::OtherFile
            })
        }
        Ok(_) => Ok(Marker::Absent),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Marker::Absent),
        Err(e) => Err(e.into()),
    }
}
pub fn discover(root: &Path, excluded: &[PathBuf], cancel: &Cancel) -> Result<Report> {
    let root = fs::canonicalize(root)?;
    ensure!(root.is_dir(), i18n::tr!(ScanRootIsNotADirectory));
    let excluded = excluded
        .iter()
        .map(|path| {
            let path = fs::canonicalize(if path.is_absolute() {
                path.clone()
            } else {
                root.join(path)
            })?;
            ensure!(path.is_dir(), i18n::tr!(ScanRootIsNotADirectory));
            ensure!(path.starts_with(&root), i18n::tr!(ScanExclusionOutsideRoot));
            Ok(path)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut report = Report {
        root: root.clone(),
        scanned_at: state::now(),
        excluded_roots: excluded.clone(),
        repositories: Vec::new(),
        non_repository_markers: Vec::new(),
    };
    if excluded.iter().any(|x| root.starts_with(x)) {
        return Ok(report);
    }
    let mut initial = None;
    if marker(&root)? != Marker::Repository {
        for ancestor in root.ancestors().skip(1) {
            if marker(ancestor)? == Marker::Repository {
                initial = Some(RepositoryScope::open(ancestor)?);
                break;
            }
        }
    }
    visit(&root, initial.as_ref(), &mut report, cancel)?;
    report.repositories.sort_by(|a, b| a.path().cmp(b.path()));
    report.non_repository_markers.sort();
    Ok(report)
}
fn visit(
    dir: &Path,
    context: Option<&RepositoryScope>,
    report: &mut Report,
    cancel: &Cancel,
) -> Result<()> {
    cancel.check()?;
    if report.excluded_roots.iter().any(|x| dir.starts_with(x)) {
        return Ok(());
    }
    if let Some(context) = context
        && !context.allows(dir)?
    {
        return Ok(());
    }
    match marker(dir)? {
        Marker::Repository => match RepositoryScope::open(dir) {
            Ok(scope) => {
                report.repositories.push(Record::Scanned {
                    path: dir.to_owned(),
                });
                children(dir, Some(&scope), report, cancel)
            }
            Err(e) => {
                report.repositories.push(Record::Failed {
                    path: dir.to_owned(),
                    error: format!("{e:#}"),
                });
                Ok(())
            }
        },
        Marker::OtherFile => {
            report.non_repository_markers.push(dir.join(".git"));
            children(dir, context, report, cancel)
        }
        Marker::Absent => children(dir, context, report, cancel),
    }
}
fn children(
    dir: &Path,
    context: Option<&RepositoryScope>,
    report: &mut Report,
    cancel: &Cancel,
) -> Result<()> {
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_name() != ".git" && entry.file_type()?.is_dir() {
            names.push(entry.file_name())
        }
    }
    names.sort();
    for name in names {
        cancel.check()?;
        visit(&dir.join(name), context, report, cancel)?;
    }
    Ok(())
}
