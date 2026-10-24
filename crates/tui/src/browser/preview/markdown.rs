//! Markdown layout preserves semantic link ranges alongside styled graphemes.
use engine::cancel::Cancel;
use std::path::Path;
pub(super) fn prepare(
    source: &str,
    width: u16,
    base: &Path,
    cancel: &Cancel,
) -> anyhow::Result<crate::links::Text> {
    cancel.check()?;
    let text = crate::markdown::linked(source, width, Some(base));
    let mut lines = Vec::new();
    for line in text.lines {
        cancel.check()?;
        lines.extend(line.wrap(usize::from(width.max(1)), true));
    }
    Ok(crate::links::Text { lines })
}
