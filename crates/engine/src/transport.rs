//! Network and write operations deliberately delegated to system Git.
use crate::{cancel::Cancel, domain::PushTarget, process};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, ffi::OsString, path::Path, time::Duration};
#[derive(Debug, Clone)]
pub struct Git {
    pub timeout: Duration,
}
impl Default for Git {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
        }
    }
}
impl Git {
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
        self.run(
            path,
            strings(&[
                "-c",
                &format!("remote.{}.mirror=false", target.remote),
                "-c",
                "push.followTags=false",
                "push",
                "--porcelain",
                "--no-force",
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
}
fn strings(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}
