//! A file review contains either styled text or an immutable raster comparison.
use crate::{diff, preview};
use anyhow::Result;
use engine::{
    cancel::Cancel,
    local::{
        files::{
            Area, Location,
            patch::{Document, Source},
        },
        history::Base,
    },
};

pub(super) fn title(location: &Location) -> String {
    match location {
        Location::Path(path) => path.display().to_string(),
        Location::Rename { before, after } | Location::Copy { before, after } => {
            format!("{} → {}", before.display(), after.display())
        }
    }
}

#[derive(Debug)]
pub(super) enum View {
    Text(Box<diff::View>),
    Images(Box<preview::View>),
}
impl View {
    #[cfg(test)]
    pub fn labels(&self) -> (String, String) {
        match self {
            Self::Text(view) => view.labels(),
            Self::Images(view) => view.labels.clone(),
        }
    }
}
pub(super) fn prepare_change(
    title: &str,
    document: Document,
    change: &engine::local::files::Change,
    cancel: &Cancel,
) -> Result<Box<View>> {
    if change.kind == 'U' {
        prepare_with(title, document, diff::Comparison::Conflict, cancel)
    } else {
        prepare(title, document, change.area, cancel)
    }
}
pub(super) fn prepare(
    title: &str,
    document: Document,
    area: Area,
    cancel: &Cancel,
) -> Result<Box<View>> {
    let comparison = match area {
        Area::Staged => diff::Comparison::Staged,
        Area::Unstaged | Area::Untracked => diff::Comparison::Working,
    };
    prepare_with(title, document, comparison, cancel)
}
pub(super) fn prepare_history(
    title: &str,
    document: Document,
    before: Base,
    after: engine::local::history::Oid,
    cancel: &Cancel,
) -> Result<Box<View>> {
    prepare_with(
        title,
        document,
        diff::Comparison::History { before, after },
        cancel,
    )
}
fn prepare_with(
    title: &str,
    document: Document,
    comparison: diff::Comparison,
    cancel: &Cancel,
) -> Result<Box<View>> {
    if let Document::Patch { files, .. } = &document
        && files.iter().any(|file| {
            matches!(file.before, Source::Image(_)) || matches!(file.after, Source::Image(_))
        })
    {
        let Document::Patch { files, .. } = document else {
            unreachable!()
        };
        return Ok(Box::new(View::Images(Box::new(preview::prepare(
            title,
            files,
            comparison.labels(),
            cancel,
        )?))));
    }
    let text = match comparison {
        diff::Comparison::Staged => diff::prepare(title, document, Area::Staged, cancel)?,
        diff::Comparison::Working => diff::prepare(title, document, Area::Unstaged, cancel)?,
        diff::Comparison::Conflict => {
            diff::prepare_comparison(title, document, comparison, cancel)?
        }
        diff::Comparison::History { before, after } => {
            diff::prepare_history(title, document, before, after, cancel)?
        }
    };
    Ok(Box::new(View::Text(text)))
}
