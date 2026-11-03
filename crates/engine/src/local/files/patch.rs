//! Structured patch lines and explicitly bounded source snapshots.
use std::{num::NonZeroU32, path::PathBuf};

pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;

#[derive(Debug)]
pub enum Document {
    Notice(String),
    Patch {
        files: Vec<FilePatch>,
        extent: Extent,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extent {
    Complete,
    Truncated,
}

#[derive(Debug)]
pub struct FilePatch {
    pub before: Source,
    pub after: Source,
    pub lines: Vec<Line>,
}

#[derive(Debug)]
pub enum Source {
    Image(super::media::Image),
    Absent,
    Text { path: PathBuf, text: String },
    Unavailable(SourceOmission),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceOmission {
    Binary,
    TooLarge,
    NonRegular,
}

#[derive(Debug, Clone)]
pub enum Line {
    Metadata {
        kind: Metadata,
        text: String,
    },
    Context {
        old: NonZeroU32,
        new: NonZeroU32,
        text: String,
    },
    Added {
        new: NonZeroU32,
        text: String,
    },
    Removed {
        old: NonZeroU32,
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Metadata {
    File,
    Hunk,
    Notice,
    NoNewline,
    Binary {
        before: BinaryPath,
        after: BinaryPath,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryPath {
    Absent,
    File(PathBuf),
}

impl Line {
    pub fn content(&self) -> &str {
        match self {
            Self::Metadata { text, .. }
            | Self::Context { text, .. }
            | Self::Added { text, .. }
            | Self::Removed { text, .. } => text,
        }
    }
    pub fn prefix(&self) -> &str {
        match self {
            Self::Context { .. } => " ",
            Self::Added { .. } => "+",
            Self::Removed { .. } => "-",
            Self::Metadata { .. } => "",
        }
    }
}

impl Document {
    /// Render the patch without terminal styles for non-UI callers.
    pub fn text(&self) -> String {
        match self {
            Self::Notice(text) => text.clone(),
            Self::Patch { files, extent } => {
                let mut text = String::new();
                for file in files {
                    for line in &file.lines {
                        text.push_str(line.prefix());
                        text.push_str(line.content());
                    }
                }
                if *extent == Extent::Truncated {
                    text.push_str(i18n::tr!(DiffExceeds128KibOnlyTheBeginning));
                }
                text
            }
        }
    }
}
