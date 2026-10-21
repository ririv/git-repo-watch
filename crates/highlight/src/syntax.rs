//! Deterministic syntax selection from explicit names, compound suffixes and first lines.
use anyhow::{Context, Result};
use std::{path::Path, sync::LazyLock};
use syntect::parsing::{SyntaxReference, SyntaxSet};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(|| {
    syntect::dumps::from_uncompressed_data(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/syntaxes.packdump"
    )))
    .expect("build-time validated syntax pack")
});

pub(crate) fn set() -> &'static SyntaxSet {
    &SYNTAXES
}

pub(crate) fn detect<'a>(
    syntaxes: &'a SyntaxSet,
    path: &Path,
    first_line: Option<&str>,
) -> Result<Option<&'a SyntaxReference>> {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("Source filename is not UTF-8")?;
    if let Some(name) = explicit_name(filename) {
        return Ok(Some(
            syntaxes
                .find_syntax_by_name(name)
                .with_context(|| format!("Bundled syntax is missing: {name}"))?,
        ));
    }
    // 先查完整文件名，再从最长复合后缀逐步缩短，避免 h.in、html.j2 被 in、j2 抢先匹配。
    for suffix in std::iter::once(filename).chain(
        filename
            .match_indices('.')
            .map(|(index, _)| &filename[index + 1..])
            .filter(|suffix| !suffix.is_empty()),
    ) {
        if let Some(syntax) = syntaxes.find_syntax_by_extension(suffix) {
            return Ok(Some(syntax));
        }
    }
    Ok(first_line.and_then(|first| syntaxes.find_syntax_by_first_line(first)))
}

fn explicit_name(filename: &str) -> Option<&'static str> {
    let lower = filename.to_ascii_lowercase();
    if ["dockerfile", "containerfile"].iter().any(|base| {
        lower == *base
            || lower
                .strip_prefix(base)
                .is_some_and(|suffix| suffix.starts_with('.') && suffix.len() > 1)
    }) {
        return Some("Dockerfile");
    }
    if lower.ends_with(".json")
        && ["tsconfig", "jsconfig"].iter().any(|base| {
            lower
                .strip_prefix(base)
                .is_some_and(|suffix| suffix.starts_with('.'))
        })
    {
        return Some("JSONC");
    }
    None
}
