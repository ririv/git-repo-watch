//! Rebuild an unresolved result from immutable index inputs, never from HEAD alone.
use super::*;
use git2::{AttrCheckFlags, AttrValue, IndexEntry, MergeFileInput, MergeFileOptions};

impl Snapshot {
    pub fn reset(&self, cancel: &Cancel) -> Result<Plan> {
        cancel.check()?;
        let repo = open(&self.files.path)?;
        super::super::super::validate(&repo, &self.files)?;
        let path = self.change.location.name();
        capture::require_supported_filters(&repo, path)?;
        let flags = AttrCheckFlags::FILE_THEN_INDEX;
        ensure!(
            matches!(
                AttrValue::from_bytes(repo.get_attr_bytes(path, "ident", flags)?),
                AttrValue::Unspecified | AttrValue::False
            ),
            i18n::tr!(ConflictResetUnsupported)
        );
        match AttrValue::from_bytes(repo.get_attr_bytes(path, "merge", flags)?) {
            AttrValue::Unspecified | AttrValue::True | AttrValue::String("text") => {}
            _ => anyhow::bail!(i18n::tr!(ConflictResetUnsupported)),
        }
        match repo.config()?.get_string("merge.default") {
            Ok(driver) => ensure!(driver == "text", i18n::tr!(ConflictResetUnsupported)),
            Err(error) if error.code() == git2::ErrorCode::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let git_path = crate::local::git_path_bytes(path)?;
        let stages = repo.index()?.conflict_get(path)?;
        ensure!(
            [&stages.ancestor, &stages.our, &stages.their]
                .iter()
                .all(|entry| entry
                    .as_ref()
                    .is_none_or(|entry| entry.path == git_path.as_ref())),
            i18n::tr!(ConflictResetUnsupported)
        );
        let current = stages.our.context(i18n::tr!(ConflictResetUnsupported))?;
        let incoming = stages.their.context(i18n::tr!(ConflictResetUnsupported))?;
        let base = stages
            .ancestor
            .map(|entry| input(&repo, entry, cancel))
            .transpose()?;
        let left = input(&repo, current, cancel)?;
        let right = input(&repo, incoming, cancel)?;
        let mut ancestor = MergeFileInput::new();
        ancestor
            .content(base.as_ref().map_or(b"", |bytes| bytes.as_slice()))
            .path(path);
        let mut ours = MergeFileInput::new();
        ours.content(&left).path(path);
        let mut theirs = MergeFileInput::new();
        theirs.content(&right).path(path);
        let mut options = MergeFileOptions::new();
        options
            .marker_size(self.marker_size as u16)
            .ancestor_label("base")
            .our_label("HEAD")
            .their_label("incoming");
        match repo.config()?.get_string("merge.conflictstyle") {
            Ok(style) => match style.as_str() {
                "merge" => {
                    options.style_standard(true);
                }
                "diff3" => {
                    options.style_diff3(true);
                }
                "zdiff3" => {
                    options.style_zdiff3(true);
                }
                _ => anyhow::bail!(i18n::tr!(ConflictResetUnsupported)),
            },
            Err(error) if error.code() == git2::ErrorCode::NotFound => {
                options.style_standard(true);
            }
            Err(error) => return Err(error.into()),
        }
        cancel.check()?;
        let result = git2::merge_file(&ancestor, &ours, &theirs, Some(&mut options))?;
        let data = std::str::from_utf8(result.content())?;
        ensure!(
            data.len() <= crate::local::files::patch::MAX_SOURCE_BYTES
                && data.lines().count() <= 50_000,
            i18n::tr!(ConflictResolveUnsupported)
        );
        let blocks = markers::parse(data, self.marker_size, cancel)?;
        ensure!(!blocks.is_empty(), i18n::tr!(ConflictResetUnsupported));
        let Version::Text { mode, .. } = &self.working else {
            unreachable!("captured regular text")
        };
        let plan = Plan {
            intent: Intent::Reset,
            files: self.files.clone(),
            change: self.change.clone(),
            before: self.working.clone(),
            after: Version::Text {
                data: data.into(),
                mode: *mode,
            },
        };
        // 预览也验证当前原始字节；确认执行仍会再次检查。
        plan.validate(&repo, cancel)?;
        Ok(plan)
    }
}
fn input(repo: &git2::Repository, mut entry: IndexEntry, cancel: &Cancel) -> Result<Vec<u8>> {
    cancel.check()?;
    ensure!(
        matches!(entry.mode, 0o100644 | 0o100755),
        i18n::tr!(ConflictResetUnsupported)
    );
    let blob = repo.find_blob(entry.id)?;
    ensure!(
        blob.size() <= crate::local::files::patch::MAX_SOURCE_BYTES && !blob.is_binary(),
        i18n::tr!(ConflictResetUnsupported)
    );
    let path = std::path::PathBuf::from(std::str::from_utf8(&entry.path)?);
    entry.flags &= !(3 << 12);
    entry.flags_extended = 0;
    let mut index = git2::Index::new()?;
    index.add(&entry)?;
    let directory = tempfile::tempdir()?;
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout
        .target_dir(directory.path())
        .force()
        .update_index(false)
        .refresh(false)
        .disable_pathspec_match(true)
        .path(&path);
    // 仅在临时目录转换换行和内置过滤，实际索引与工作区保持原样。
    repo.checkout_index(Some(&mut index), Some(&mut checkout))?;
    let converted = capture::working(&directory.path().join(path))?;
    let Version::Text { data, .. } = converted else {
        anyhow::bail!(i18n::tr!(ConflictResetUnsupported))
    };
    Ok(data.as_bytes().to_vec())
}
impl Plan {
    pub fn reset_review(&self) -> Result<String> {
        let mut patch = git2::Patch::from_buffers(
            self.before.text().as_bytes(),
            Some(self.file()),
            self.after.text().as_bytes(),
            Some(self.file()),
            None,
        )?;
        let diff = patch.to_buf()?;
        Ok(format!(
            "{}\n\n{}\n\n{}",
            i18n::tr!(ConflictResetConfirm),
            self.file().display(),
            std::str::from_utf8(diff.as_ref())?
        ))
    }
}
