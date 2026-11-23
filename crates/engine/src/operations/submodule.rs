//! Reviewed Git submodule operations.
pub mod checkout;
use crate::{
    cancel::Cancel,
    local::{guard::Position, idle, open, submodules},
    platform::files::is_link,
    transport::Git,
};
use anyhow::{Context, Result, ensure};
use git2::{ObjectType, Oid, Repository, Status};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

const MANIFEST: &str = ".gitmodules";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Add {
    source: String,
    path: PathBuf,
}
impl Add {
    pub fn new(source: String, path: &Path) -> Result<Self> {
        ensure!(
            !source.trim().is_empty() && !source.chars().any(char::is_control),
            i18n::tr!(SubmoduleAddUsage)
        );
        Ok(Self {
            source,
            path: submodules::relative_directory(path)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    repository: PathBuf,
    git_dir: PathBuf,
    head: Position,
    manifest: Option<(Oid, Oid)>,
    destination: Destination,
    add: Add,
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum Destination {
    Missing,
    Empty,
    Checkout { git_dir: PathBuf, head: Position },
}
impl Plan {
    pub fn repository(&self) -> &Path {
        &self.repository
    }
    pub fn source(&self) -> &str {
        &self.add.source
    }
    pub fn directory(&self) -> &Path {
        &self.add.path
    }
}

pub(crate) fn prepare(repository: &Path, add: Add, cancel: &Cancel) -> Result<Plan> {
    cancel.check()?;
    let repo = open(repository)?;
    idle(&repo)?;
    let directory = submodules::checked_directory(repository, &add.path)?;
    ensure!(
        repo.index()?.get_path(&add.path, 0).is_none(),
        i18n::tr!(SubmoduleDestinationTracked)
    );
    let destination = match fs::read_dir(&directory) {
        Ok(mut entries) => {
            if entries.next().transpose()?.is_none() {
                Destination::Empty
            } else {
                let checkout = open(&directory).context(i18n::tr!(SubmoduleDestinationOccupied))?;
                Destination::Checkout {
                    git_dir: checkout.path().canonicalize()?,
                    head: Position::read(&checkout)?,
                }
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Destination::Missing,
        Err(error) => return Err(error.into()),
    };
    Ok(Plan {
        repository: repository.to_owned(),
        git_dir: repo.path().canonicalize()?,
        head: Position::read(&repo)?,
        manifest: manifest(&repo, repository)?,
        destination,
        add,
    })
}

fn manifest(repo: &Repository, root: &Path) -> Result<Option<(Oid, Oid)>> {
    let path = root.join(MANIFEST);
    let staged = repo.index()?.get_path(Path::new(MANIFEST), 0);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound && staged.is_none() => return Ok(None),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            anyhow::bail!(i18n::tr!(SubmoduleManifestModified));
        }
        Err(error) => return Err(error.into()),
    };
    let staged = staged.context(i18n::tr!(SubmoduleManifestModified))?;
    ensure!(
        metadata.is_file() && !is_link(&metadata),
        i18n::tr!(SubmoduleManifestModified)
    );
    let status = repo.status_file(Path::new(MANIFEST))?;
    ensure!(
        !status.intersects(
            Status::WT_NEW
                | Status::WT_MODIFIED
                | Status::WT_DELETED
                | Status::WT_TYPECHANGE
                | Status::WT_RENAMED
                | Status::WT_UNREADABLE
        ),
        i18n::tr!(SubmoduleManifestModified)
    );
    // 允许连续新增子模块，保护暂存区中已有的 .gitmodules 内容和预览时的工作区版本。
    let worktree = Oid::hash_object(ObjectType::Blob, &fs::read(path)?)?;
    Ok(Some((staged.id, worktree)))
}

pub(crate) fn execute(git: &Git, plan: &Plan, cancel: &Cancel) -> Result<()> {
    let current = prepare(&plan.repository, plan.add.clone(), cancel)?;
    ensure!(current == *plan, i18n::tr!(MutationChanged));
    git.add_submodule(&plan.repository, plan.source(), plan.directory(), cancel)
}
