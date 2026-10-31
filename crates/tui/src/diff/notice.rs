//! Render binary paths from structured Git data while preserving the raw patch.
use engine::local::files::patch::BinaryPath;

pub(super) fn binary(before: &BinaryPath, after: &BinaryPath) -> String {
    let path = |side: &BinaryPath, prefix| match side {
        BinaryPath::Absent => "/dev/null".to_owned(),
        BinaryPath::File(path) => format!("{prefix}/{}", super::safe(&path.display().to_string())),
    };
    format!(
        "Binary files {} and {} differ",
        path(before, "a"),
        path(after, "b")
    )
}
