//! Hard reset previews capture tracked edits and untracked paths obstructing the target tree.
use crate::{
    cancel::Cancel,
    local::{
        files::{self, Area, Change},
        guard::Guard,
    },
};
use anyhow::{Result, ensure};
use git2::{Index, ObjectType, Repository, Tree};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs::{self, Metadata, OpenOptions},
    io::{ErrorKind, Read},
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Component, Path, PathBuf},
};

type Obstructions = BTreeMap<PathBuf, [u8; 32]>;

#[derive(Debug, Clone)]
pub struct Review {
    guard: Guard,
    changes: Vec<Change>,
    obstructions: Obstructions,
}

impl Review {
    pub(super) fn capture(
        path: &Path,
        repo: &Repository,
        tree: &Tree,
        cancel: &Cancel,
    ) -> Result<Self> {
        let guard = Guard::capture_reset(path, cancel)?;
        let index = repo.index()?;
        let changes = files::read(path)?
            .changes
            .into_iter()
            .filter(|change| match change.area {
                Area::Staged => true,
                Area::Untracked => false,
                Area::Unstaged => !index
                    .get_path(change.location.name(), 0)
                    .is_some_and(|entry| entry.mode == 0o160000),
            })
            .collect();
        let obstructions = obstructions(path, repo, tree, cancel)?;
        ensure!(
            Guard::capture_reset(path, cancel)? == guard,
            i18n::tr!(MutationChanged)
        );
        Ok(Self {
            guard,
            changes,
            obstructions,
        })
    }

    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    pub fn obstructions(&self) -> impl ExactSizeIterator<Item = &Path> {
        self.obstructions.keys().map(PathBuf::as_path)
    }

    pub(super) fn validate(
        &self,
        path: &Path,
        repo: &Repository,
        target: &Tree,
        cancel: &Cancel,
    ) -> Result<()> {
        ensure!(
            obstructions(path, repo, target, cancel)? == self.obstructions
                && Guard::capture_reset(path, cancel)? == self.guard,
            i18n::tr!(MutationChanged)
        );
        Ok(())
    }
}

fn obstructions(
    path: &Path,
    repo: &Repository,
    tree: &Tree,
    cancel: &Cancel,
) -> Result<Obstructions> {
    let mut found = Obstructions::new();
    let index = repo.index()?;
    inspect(path, Path::new(""), repo, &index, tree, cancel, &mut found)?;
    Ok(found)
}

fn inspect(
    root: &Path,
    parent: &Path,
    repo: &Repository,
    index: &Index,
    tree: &Tree,
    cancel: &Cancel,
    found: &mut Obstructions,
) -> Result<()> {
    for entry in tree.iter() {
        cancel.check()?;
        let name = Path::new(OsStr::from_bytes(entry.name_bytes()));
        let mut components = name.components();
        ensure!(
            matches!(components.next(), Some(Component::Normal(part)) if Path::new(part) == name)
                && components.next().is_none(),
            i18n::tr!(MutationChanged)
        );
        let relative = parent.join(name);
        let full = root.join(&relative);
        let metadata = match fs::symlink_metadata(&full) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        let kind = entry.kind();
        if kind == Some(ObjectType::Tree) && metadata.is_dir() {
            let tree = repo.find_tree(entry.id())?;
            inspect(root, &relative, repo, index, &tree, cancel, found)?;
        } else if !(kind == Some(ObjectType::Commit) && metadata.is_dir())
            && (metadata.is_dir() || index.get_path(&relative, 0).is_none())
        {
            let mut digest = Sha256::new();
            hash_path(&mut digest, &full, &metadata, cancel)?;
            found.insert(relative, digest.finalize().into());
        }
    }
    Ok(())
}

fn hash_path(digest: &mut Sha256, path: &Path, metadata: &Metadata, cancel: &Cancel) -> Result<()> {
    cancel.check()?;
    digest.update(metadata.mode().to_le_bytes());
    if metadata.is_symlink() {
        digest.update(b"link");
        let target = fs::read_link(path)?;
        let bytes = target.as_os_str().as_encoded_bytes();
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    } else if metadata.is_file() {
        digest.update(b"file");
        digest.update(metadata.len().to_le_bytes());
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)?;
        let mut buffer = vec![0; 65536];
        loop {
            cancel.check()?;
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
    } else if metadata.is_dir() {
        digest.update(b"dir");
        let mut entries = fs::read_dir(path)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        digest.update((entries.len() as u64).to_le_bytes());
        for entry in entries {
            let name = entry.file_name();
            let bytes = name.as_encoded_bytes();
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(bytes);
            let path = entry.path();
            hash_path(digest, &path, &fs::symlink_metadata(&path)?, cancel)?;
        }
    } else {
        anyhow::bail!(i18n::tr!(MutationNonRegularFile));
    }
    Ok(())
}
