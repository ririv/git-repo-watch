//! Index identity includes unresolved stages without writing an invalid tree.
use anyhow::Result;
use git2::{Index, Oid};
use sha2::{Digest, Sha256};
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Stamp {
    Tree(Oid),
    Conflicts([u8; 32]),
}
impl Stamp {
    pub fn read(index: &mut Index) -> Result<Self> {
        if !index.has_conflicts() {
            return Ok(Self::Tree(index.write_tree()?));
        }
        let mut hash = Sha256::new();
        for entry in index.iter() {
            hash.update((entry.path.len() as u64).to_le_bytes());
            hash.update(entry.path);
            hash.update(entry.id.as_bytes());
            hash.update(entry.mode.to_le_bytes());
            hash.update(entry.flags.to_le_bytes());
            hash.update(entry.flags_extended.to_le_bytes());
        }
        Ok(Self::Conflicts(hash.finalize().into()))
    }
    pub fn tree(&self) -> Result<Oid> {
        match self {
            Self::Tree(oid) => Ok(*oid),
            Self::Conflicts(_) => anyhow::bail!(i18n::tr!(RepositoryHasUnresolvedConflicts)),
        }
    }
}
