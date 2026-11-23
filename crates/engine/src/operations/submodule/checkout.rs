//! Review pinned submodule checkouts and protect existing working copies.
use crate::{
    cancel::Cancel,
    local::{self, guard::Position, submodules},
    transport::Git,
};
use anyhow::{Context, Result, ensure};
use git2::{Oid, Repository, StatusOptions, Submodule};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Initialize,
    Update,
}
impl Mode {
    pub fn title(self) -> &'static str {
        match self {
            Self::Initialize => i18n::tr!(SubmoduleInitialize),
            Self::Update => i18n::tr!(SubmoduleUpdate),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    mode: Mode,
    scope: Scope,
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum Scope {
    All,
    Path(PathBuf),
}
impl Request {
    pub fn new(mode: Mode, path: Option<&Path>) -> Result<Self> {
        Ok(Self {
            mode,
            scope: match path {
                Some(path) => Scope::Path(submodules::relative_directory(path)?),
                None => Scope::All,
            },
        })
    }
    fn selects(&self, path: &Path) -> bool {
        match &self.scope {
            Scope::All => true,
            Scope::Path(selected) => selected == path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    repository: PathBuf,
    git_dir: PathBuf,
    head: Position,
    request: Request,
    targets: Vec<Target>,
}
impl Plan {
    pub fn repository(&self) -> &Path {
        &self.repository
    }
    pub fn mode(&self) -> Mode {
        self.request.mode
    }
    pub fn targets(&self) -> &[Target] {
        &self.targets
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub path: PathBuf,
    pub pinned: Oid,
    url: Option<Vec<u8>>,
    checkout: Checkout,
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum Checkout {
    Missing,
    Present {
        git_dir: PathBuf,
        head: Position,
        children: Vec<Target>,
    },
}
impl Target {
    pub fn current(&self) -> Option<Oid> {
        match &self.checkout {
            Checkout::Missing => None,
            Checkout::Present { head, .. } => head.oid().ok(),
        }
    }
}

pub(crate) fn prepare(repository: &Path, request: Request, cancel: &Cancel) -> Result<Plan> {
    cancel.check()?;
    let repo = local::open(repository)?;
    local::idle(&repo)?;
    let mut targets = Vec::new();
    for module in repo.submodules()? {
        cancel.check()?;
        if request.selects(module.path())
            && let Some(target) = inspect(repository, &module, request.mode, false, cancel)?
        {
            targets.push(target);
        }
    }
    targets.sort_by(|left, right| left.path.cmp(&right.path));
    if targets.is_empty() {
        anyhow::bail!(match (&request.scope, request.mode) {
            (Scope::All, Mode::Initialize) => i18n::tr!(NoSubmodules),
            (Scope::All, Mode::Update) => i18n::tr!(SubmoduleNoInitialized),
            (Scope::Path(_), _) => i18n::tr!(SubmodulePathNotAvailable),
        });
    }
    Ok(Plan {
        repository: repository.to_owned(),
        git_dir: repo.path().canonicalize()?,
        head: Position::read(&repo)?,
        request,
        targets,
    })
}

fn inspect(
    root: &Path,
    module: &Submodule<'_>,
    mode: Mode,
    ancestor_moves: bool,
    cancel: &Cancel,
) -> Result<Option<Target>> {
    cancel.check()?;
    let Some(pinned) = module.index_id() else {
        // 已经从索引移除的子模块不再属于这次检出。
        return Ok(None);
    };
    let path = submodules::relative_directory(module.path())?;
    let directory = submodules::checked_directory(root, &path)?;
    let checkout = match fs::symlink_metadata(directory.join(".git")) {
        Ok(_) => {
            let repo = local::open(&directory)?;
            let head = Position::read(&repo)?;
            let moves = ancestor_moves || head.oid()? != pinned;
            if moves {
                protect_worktree(&repo).with_context(|| directory.display().to_string())?;
            }
            let mut children = Vec::new();
            for child in repo.submodules()? {
                if let Some(target) = inspect(&directory, &child, mode, moves, cancel)? {
                    children.push(target);
                }
            }
            children.sort_by(|left, right| left.path.cmp(&right.path));
            Checkout::Present {
                git_dir: repo.path().canonicalize()?,
                head,
                children,
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound && mode == Mode::Update => {
            return Ok(None);
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            match fs::read_dir(&directory) {
                Ok(mut files) => ensure!(
                    files.next().transpose()?.is_none(),
                    i18n::tr!(SubmoduleDestinationOccupied)
                ),
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            Checkout::Missing
        }
        Err(error) => return Err(error.into()),
    };
    Ok(Some(Target {
        path,
        pinned,
        url: module.opt_url_bytes().map(<[u8]>::to_vec),
        checkout,
    }))
}

fn protect_worktree(repo: &Repository) -> Result<()> {
    local::idle(repo)?;
    let mut options = StatusOptions::new();
    // 嵌套子模块由同一棵树逐个检查，这里只检查仓库自己的文件。
    options.exclude_submodules(true).include_untracked(true);
    ensure!(
        repo.statuses(Some(&mut options))?.is_empty(),
        i18n::tr!(SubmoduleCheckoutDirty)
    );
    Ok(())
}

pub(crate) fn execute(git: &Git, plan: &Plan, cancel: &Cancel) -> Result<()> {
    let current = prepare(&plan.repository, plan.request.clone(), cancel)?;
    ensure!(current == *plan, i18n::tr!(MutationChanged));
    git.checkout_submodules(&plan.repository, plan.mode(), &plan.targets, cancel)?;
    let observed = prepare(&plan.repository, plan.request.clone(), cancel)?;
    verify(&plan.repository, &observed.targets)
}

fn verify(root: &Path, targets: &[Target]) -> Result<()> {
    for target in targets {
        let path = root.join(&target.path);
        ensure!(
            target.current() == Some(target.pinned),
            "{}: {}",
            i18n::tr!(SubmoduleCheckoutMismatch),
            path.display()
        );
        if let Checkout::Present { children, .. } = &target.checkout {
            verify(&path, children)?;
        }
    }
    Ok(())
}
