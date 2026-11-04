//! Network and write operations deliberately delegated to system Git.
use crate::{cancel::Cancel, domain::PushTarget, process};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, ffi::OsString, path::Path, time::Duration};
#[derive(Debug, Clone)]
pub struct Git {
    pub timeout: Duration,
}
pub(crate) enum RepositoryStash {
    Tracked,
    IncludeUntracked,
}
pub(crate) enum PushMode {
    FastForward,
    Create,
    ForceWithLease(git2::Oid),
}
impl Default for Git {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
        }
    }
}
impl Git {
    /// Resolve the editor through system Git, including repository-specific configuration.
    pub fn editor(&self, path: &Path, cancel: &Cancel) -> Result<String> {
        let data = self.run(path, strings(&["var", "GIT_EDITOR"]), cancel)?;
        Ok(std::str::from_utf8(&data)?
            .strip_suffix('\n')
            .context(i18n::tr!(ExternalEditorInvalid))?
            .to_owned())
    }

    fn run(&self, path: &Path, args: Vec<OsString>, cancel: &Cancel) -> Result<Vec<u8>> {
        process::run(
            Path::new("/usr/bin/git"),
            &args,
            Some(path),
            cancel,
            self.timeout,
        )?
        .success("git")
    }
    pub fn fetch_tracking(
        &self,
        path: &Path,
        remote: &crate::local::branches::fetch::Remote,
        cancel: &Cancel,
    ) -> Result<()> {
        let mappings = remote.arguments();
        if mappings.is_empty() {
            return Ok(());
        }
        let mut args = strings(&[
            "fetch",
            "--prune",
            "--no-prune-tags",
            "--no-tags",
            "--no-recurse-submodules",
            "--refmap=",
            "--",
            &remote.name,
        ]);
        args.extend(mappings.into_iter().map(OsString::from));
        args.extend(
            remote
                .exclusions()
                .map(|source| OsString::from(format!("^{source}"))),
        );
        self.run(path, args, cancel)?;
        Ok(())
    }
    pub fn fetch(&self, path: &Path, remote: &str, cancel: &Cancel) -> Result<()> {
        self.run(
            path,
            strings(&["fetch", "--no-recurse-submodules", "--", remote]),
            cancel,
        )?;
        Ok(())
    }
    pub fn heads(
        &self,
        path: &Path,
        remote: &str,
        cancel: &Cancel,
    ) -> Result<BTreeMap<String, git2::Oid>> {
        let data = self.run(
            path,
            strings(&["ls-remote", "--heads", "--", remote]),
            cancel,
        )?;
        let text = std::str::from_utf8(&data)?;
        let mut heads = BTreeMap::new();
        for row in text.lines() {
            let (oid, reference) = row
                .split_once('\t')
                .context(i18n::tr!(InvalidRemoteBranchRecord))?;
            ensure!(
                heads.insert(reference.to_owned(), oid.parse()?).is_none(),
                i18n::tr!(ServerReturnedADuplicateBranchReference)
            );
        }
        Ok(heads)
    }
    pub fn head(
        &self,
        path: &Path,
        remote: &str,
        reference: &str,
        cancel: &Cancel,
    ) -> Result<git2::Oid> {
        self.heads(path, remote, cancel)?
            .remove(reference)
            .context(i18n::tr!(RemoteUpstreamBranchDoesNotExistAlt))
    }
    pub fn branch_tip(
        &self,
        path: &Path,
        remote: &str,
        reference: &str,
        cancel: &Cancel,
    ) -> Result<Option<git2::Oid>> {
        Ok(self.heads(path, remote, cancel)?.remove(reference))
    }
    fn run_replay(&self, path: &Path, args: Vec<OsString>, cancel: &Cancel) -> Result<()> {
        process::run_with_environment(
            Path::new("/usr/bin/git"),
            &args,
            Some(path),
            cancel,
            self.timeout,
            &[
                ("GIT_EDITOR".into(), "/usr/bin/true".into()),
                ("GIT_SEQUENCE_EDITOR".into(), "/usr/bin/true".into()),
            ],
        )?
        .success("git")?;
        Ok(())
    }
    pub fn replay(
        &self,
        path: &Path,
        kind: crate::operations::replay::Kind,
        oid: git2::Oid,
        parent: Option<std::num::NonZeroUsize>,
        cancel: &Cancel,
    ) -> Result<()> {
        let mut args = strings(&[kind.command(), "--no-edit"]);
        if let Some(parent) = parent {
            args.extend(strings(&["--mainline", &parent.to_string()]));
        }
        args.extend(strings(&["--", &oid.to_string()]));
        self.run_replay(path, args, cancel)
    }
    pub fn replay_control(
        &self,
        path: &Path,
        kind: crate::operations::replay::Kind,
        control: crate::operations::replay::Control,
        cancel: &Cancel,
    ) -> Result<()> {
        self.run_replay(path, strings(&[kind.command(), control.argument()]), cancel)
    }
    pub fn cherry_pick_sequence(
        &self,
        path: &Path,
        commits: &[git2::Oid],
        cancel: &Cancel,
    ) -> Result<()> {
        let mut args = strings(&["cherry-pick", "--no-edit", "--"]);
        args.extend(commits.iter().map(|oid| OsString::from(oid.to_string())));
        self.run_replay(path, args, cancel)
    }
    pub(crate) fn save_stash(
        &self,
        path: &Path,
        mode: RepositoryStash,
        message: Option<&str>,
        cancel: &Cancel,
    ) -> Result<()> {
        self.stash_command(path, &stash_arguments(mode, message), None, cancel)?;
        Ok(())
    }
    pub(crate) fn clean_stashed_files(
        &self,
        plan: &crate::local::stash::SavePlan,
        index: &Path,
        base: &git2::Index,
        cancel: &Cancel,
    ) -> Result<()> {
        let (tracked, added): (Vec<_>, Vec<_>) = plan
            .files
            .iter()
            .partition(|path| base.get_path(path, 0).is_some());
        let run = |mut args: Vec<OsString>, paths: Vec<&std::path::PathBuf>| -> Result<()> {
            args.push("--".into());
            args.extend(paths.into_iter().map(|path| {
                let mut value = OsString::from(":(top,literal)");
                value.push(path.as_os_str());
                value
            }));
            self.stash_command(&plan.path, &args, Some(index), cancel)?;
            Ok(())
        };
        if !tracked.is_empty() {
            run(
                strings(&[
                    "restore",
                    "--source",
                    &plan.head_oid().to_string(),
                    "--worktree",
                ]),
                tracked,
            )?;
        }
        if !added.is_empty() {
            run(strings(&["clean", "--force", "-d", "-x"]), added)?;
        }
        Ok(())
    }
    fn stash_command(
        &self,
        path: &Path,
        args: &[OsString],
        index: Option<&Path>,
        cancel: &Cancel,
    ) -> Result<()> {
        let repo = crate::local::open(path)?;
        let worktree = repo.workdir().context(i18n::tr!(RepositoryHasNoWorktree))?;
        let current = repo.index()?;
        let index = index
            .or_else(|| current.path())
            .context(i18n::tr!(StashInvalidPath))?;
        // 显式绑定仓库、对象库和索引，避免启动环境中的 Git 上下文改变写入目标。
        let mut environment = vec![
            (
                OsString::from("GIT_DIR"),
                repo.path().as_os_str().to_owned(),
            ),
            (
                "GIT_COMMON_DIR".into(),
                repo.commondir().as_os_str().to_owned(),
            ),
            ("GIT_WORK_TREE".into(), worktree.as_os_str().to_owned()),
            ("GIT_INDEX_FILE".into(), index.as_os_str().to_owned()),
            (
                "GIT_OBJECT_DIRECTORY".into(),
                repo.commondir().join("objects").into_os_string(),
            ),
            ("GIT_NAMESPACE".into(), "".into()),
        ];
        for name in [
            "GIT_LITERAL_PATHSPECS",
            "GIT_GLOB_PATHSPECS",
            "GIT_NOGLOB_PATHSPECS",
            "GIT_ICASE_PATHSPECS",
        ] {
            environment.push((name.into(), "0".into()));
        }
        process::run_with_environment(
            Path::new("/usr/bin/git"),
            args,
            Some(worktree),
            cancel,
            self.timeout,
            &environment,
        )?
        .success("git stash")?;
        Ok(())
    }
    pub fn commit(&self, path: &Path, message: &str, cancel: &Cancel) -> Result<()> {
        self.run(path, strings(&["commit", "-m", message]), cancel)?;
        Ok(())
    }
    pub(crate) fn commit_index(
        &self,
        path: &Path,
        index: &Path,
        message: &str,
        cancel: &Cancel,
    ) -> Result<()> {
        process::run_with_environment(
            Path::new("/usr/bin/git"),
            &strings(&["commit", "-m", message]),
            Some(path),
            cancel,
            self.timeout,
            &[("GIT_INDEX_FILE".into(), index.as_os_str().into())],
        )?
        .success("git commit")?;
        Ok(())
    }
    pub fn fast_forward(&self, path: &Path, oid: git2::Oid, cancel: &Cancel) -> Result<()> {
        self.run(
            path,
            strings(&[
                "merge",
                "--ff-only",
                "--no-edit",
                "--no-autostash",
                "--no-stat",
                &oid.to_string(),
            ]),
            cancel,
        )?;
        Ok(())
    }
    pub fn post_checkout(
        &self,
        path: &Path,
        before: git2::Oid,
        after: git2::Oid,
        cancel: &Cancel,
    ) -> Result<()> {
        self.run(
            path,
            strings(&[
                "hook",
                "run",
                "--ignore-missing",
                "post-checkout",
                "--",
                &before.to_string(),
                &after.to_string(),
                "1",
            ]),
            cancel,
        )?;
        Ok(())
    }
    pub fn push(&self, path: &Path, target: &PushTarget, cancel: &Cancel) -> Result<()> {
        self.push_mode(path, target, PushMode::FastForward, cancel)
    }
    pub(crate) fn push_mode(
        &self,
        path: &Path,
        target: &PushTarget,
        mode: PushMode,
        cancel: &Cancel,
    ) -> Result<()> {
        let protection = match mode {
            PushMode::FastForward => "--no-force".to_owned(),
            PushMode::Create => format!("--force-with-lease={}:", target.remote_ref),
            PushMode::ForceWithLease(expected) => {
                format!("--force-with-lease={}:{}", target.remote_ref, expected)
            }
        };
        self.run(
            path,
            strings(&[
                "-c",
                &format!("remote.{}.mirror=false", target.remote),
                "-c",
                "push.followTags=false",
                "push",
                "--porcelain",
                &protection,
                "--no-mirror",
                "--no-follow-tags",
                "--recurse-submodules=no",
                "--",
                &target.remote,
                &format!("{}:{}", target.local_oid, target.remote_ref),
            ]),
            cancel,
        )?;
        Ok(())
    }
    pub(crate) fn set_upstream(
        &self,
        path: &Path,
        branch: &str,
        reference: &str,
        cancel: &Cancel,
    ) -> Result<()> {
        self.run(
            path,
            strings(&[
                "branch",
                &format!("--set-upstream-to={reference}"),
                "--",
                branch,
            ]),
            cancel,
        )?;
        Ok(())
    }
}
fn strings(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

fn stash_arguments(mode: RepositoryStash, message: Option<&str>) -> Vec<OsString> {
    let mut args = strings(&["stash", "push"]);
    if matches!(mode, RepositoryStash::IncludeUntracked) {
        args.push("--include-untracked".into());
    }
    if let Some(message) = message {
        args.extend(strings(&["--message", message]));
    }
    args
}
