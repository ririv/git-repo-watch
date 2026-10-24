//! Visible raw references supplement explicit Markdown anchors without guessing relative paths.
use engine::desktop::Target;
use std::{ops::Range, path::PathBuf};
pub(super) fn references(text: &str, paths: &[PathBuf]) -> Vec<(Range<usize>, Target)> {
    let mut result = Vec::new();
    for path in paths {
        let Some(label) = path.to_str() else {
            continue;
        };
        let Some(target) = Target::path(path.clone()) else {
            continue;
        };
        for (index, _) in text.match_indices(label) {
            let previous = text[..index].chars().next_back();
            let next = text[index + label.len()..].chars().next();
            if previous.is_some_and(|c| c.is_alphanumeric() || c == '/')
                || next.is_some_and(|c| c.is_alphanumeric() || "/_-.".contains(c))
            {
                continue;
            }
            result.push((index..index + label.len(), target.clone()));
        }
    }
    for prefix in ["https://", "http://", "file://"] {
        for (start, _) in text.match_indices(prefix) {
            let rest = &text[start..];
            let len = rest
                .find(|c: char| c.is_whitespace() || "\"'<>`）】》。，；！？".contains(c))
                .unwrap_or(rest.len());
            let mut value = rest[..len].trim_end_matches(['.', ',', ';', '!', '?', ']']);
            while value.ends_with(')')
                && value.chars().filter(|c| *c == ')').count()
                    > value.chars().filter(|c| *c == '(').count()
            {
                value = &value[..value.len() - 1];
            }
            if let Some(target) = Target::reference(value, None) {
                result.push((start..start + value.len(), target));
            }
        }
    }
    for (start, first) in text.char_indices().filter(|(_, c)| *c == '/' || *c == '~') {
        let previous = text[..start].chars().next_back();
        if previous.is_some_and(|c| !c.is_whitespace() && !"\"'`([".contains(c)) {
            continue;
        }
        let rest = &text[start..];
        if first == '~' && !rest.starts_with("~/") {
            continue;
        }
        let quote = previous.filter(|c| "\"'`".contains(*c));
        let len = rest
            .find(|c: char| {
                if let Some(quote) = quote {
                    c == quote
                } else {
                    c.is_whitespace() || "\"'`<>)]）】》。，；！？".contains(c)
                }
            })
            .unwrap_or(rest.len());
        let value = &rest[..len];
        if value.starts_with("//") || value.matches('/').count() < 2 {
            continue;
        }
        let path = if let Some(tail) = value.strip_prefix("~/") {
            let Some(home) = std::env::var_os("HOME") else {
                continue;
            };
            PathBuf::from(home).join(tail)
        } else {
            PathBuf::from(value)
        };
        if let Some(target) = Target::path(path) {
            result.push((start..start + len, target));
        }
    }
    result
}
