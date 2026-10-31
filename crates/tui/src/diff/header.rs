//! Interpret extended Git file headers while retaining their original text.
mod disclosure;
use anyhow::{Result, bail, ensure};
pub(super) use disclosure::Raw;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileMode {
    Regular,
    Executable,
    Symlink,
    Submodule,
}
impl FileMode {
    fn parse(text: &str) -> Result<Self> {
        Ok(match text {
            "100644" => Self::Regular,
            "100755" => Self::Executable,
            "120000" => Self::Symlink,
            "160000" => Self::Submodule,
            _ => bail!("{}: {text}", i18n::tr!(InvalidDiffStructure)),
        })
    }
    fn label(self) -> String {
        let (name, mode) = match self {
            Self::Regular => (i18n::tr!(DiffModeRegular), "100644"),
            Self::Executable => (i18n::tr!(DiffModeExecutable), "100755"),
            Self::Symlink => (i18n::tr!(DiffModeSymlink), "120000"),
            Self::Submodule => (i18n::tr!(DiffModeSubmodule), "160000"),
        };
        format!("{name} ({mode})")
    }
}

#[derive(Debug)]
pub(crate) enum Fact {
    Text(String),
    Field { label: &'static str, value: String },
}
#[cfg(test)]
impl Fact {
    pub fn text(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Field { label, value } => format!("{label}: {value}"),
        }
    }
}

pub(crate) fn summary(facts: &[Fact]) -> ratatui::text::Line<'static> {
    use crate::view::{ACCENT, MUTED, safe};
    use ratatui::{
        style::Style,
        text::{Line, Span},
    };
    let mut spans = Vec::new();
    for (index, fact) in facts.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" · ", Style::default().fg(MUTED)));
        }
        let value = match fact {
            Fact::Text(text) => text,
            Fact::Field { label, value } => {
                spans.push(Span::styled(
                    format!("{label}: "),
                    Style::default().fg(MUTED),
                ));
                value
            }
        };
        spans.push(Span::styled(safe(value), Style::default().fg(ACCENT)));
    }
    Line::from(spans)
}

pub(super) fn facts(lines: &[String]) -> Result<Vec<Fact>> {
    let mut facts = Vec::new();
    let mut lines = lines.iter();
    while let Some(line) = lines.next() {
        let fact = if let Some(mode) = line.strip_prefix("new file mode ") {
            Fact::Text(match FileMode::parse(mode)? {
                FileMode::Regular => i18n::tr!(DiffHeaderAdded).into(),
                mode => format!("{} · {}", i18n::tr!(DiffHeaderAdded), mode.label()),
            })
        } else if let Some(mode) = line.strip_prefix("deleted file mode ") {
            Fact::Text(format!(
                "{} · {}",
                i18n::tr!(DiffHeaderDeleted),
                FileMode::parse(mode)?.label()
            ))
        } else if let Some(before) = line.strip_prefix("old mode ") {
            let after = lines
                .next()
                .and_then(|line| line.strip_prefix("new mode "))
                .ok_or_else(|| anyhow::anyhow!(i18n::tr!(InvalidDiffStructure)))?;
            Fact::Field {
                label: i18n::tr!(DiffHeaderMode),
                value: format!(
                    "{} → {}",
                    FileMode::parse(before)?.label(),
                    FileMode::parse(after)?.label()
                ),
            }
        } else if line.starts_with("rename from ") {
            facts.insert(0, Fact::Text(i18n::tr!(DiffHeaderRenamed).into()));
            continue;
        } else if line.starts_with("copy from ") {
            facts.insert(0, Fact::Text(i18n::tr!(DiffHeaderCopied).into()));
            continue;
        } else if let Some(value) = line.strip_prefix("similarity index ") {
            percentage(i18n::tr!(DiffHeaderSimilarity), value)?
        } else if let Some(value) = line.strip_prefix("dissimilarity index ") {
            percentage(i18n::tr!(DiffHeaderDissimilarity), value)?
        } else if [
            "diff --git ",
            "index ",
            "--- ",
            "+++ ",
            "rename to ",
            "copy to ",
        ]
        .iter()
        .any(|prefix| line.starts_with(prefix))
        {
            continue;
        } else {
            // 未归类的扩展头原样展示，避免隐藏 Git 提供的额外信息。
            Fact::Text(line.clone())
        };
        facts.push(fact);
    }
    Ok(facts)
}

fn percentage(label: &'static str, value: &str) -> Result<Fact> {
    let number: u8 = value
        .strip_suffix('%')
        .ok_or_else(|| anyhow::anyhow!(i18n::tr!(InvalidDiffStructure)))?
        .parse()?;
    ensure!(number <= 100, i18n::tr!(InvalidDiffStructure));
    Ok(Fact::Field {
        label,
        value: format!("{number}%"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_files_only_describe_non_default_modes_in_both_languages() {
        for language in [i18n::Language::English, i18n::Language::Chinese] {
            let _language = i18n::using(language);
            let added = i18n::tr!(DiffHeaderAdded);
            for (mode, expected) in [
                ("100644", added.to_owned()),
                (
                    "100755",
                    format!("{added} · {} (100755)", i18n::tr!(DiffModeExecutable)),
                ),
                (
                    "120000",
                    format!("{added} · {} (120000)", i18n::tr!(DiffModeSymlink)),
                ),
                (
                    "160000",
                    format!("{added} · {} (160000)", i18n::tr!(DiffModeSubmodule)),
                ),
            ] {
                let facts = facts(&[format!("new file mode {mode}")]).unwrap();
                let text = facts.iter().map(Fact::text).collect::<Vec<_>>().join(" · ");
                assert_eq!(text, expected);
            }
        }
    }
    #[test]
    fn extended_headers_describe_permissions_rename_copy_and_similarity_in_both_languages() {
        for language in [i18n::Language::English, i18n::Language::Chinese] {
            let _language = i18n::using(language);
            let source = [
                "diff --git a/old b/new",
                "index 0123456..abcdef0 100644",
                "old mode 100644",
                "new mode 100755",
                "similarity index 98%",
                "rename from old",
                "rename to new",
                "dissimilarity index 10%",
                "copy from source",
                "copy to target",
                "--- a/old",
                "+++ b/new",
            ]
            .map(str::to_owned);
            let facts = facts(&source).unwrap();
            let text = facts.iter().map(Fact::text).collect::<Vec<_>>().join(" · ");
            assert!(text.contains(i18n::tr!(DiffModeExecutable)));
            assert!(text.contains("100644") && text.contains("100755"));
            assert!(text.contains(i18n::tr!(DiffHeaderRenamed)));
            assert!(text.contains(i18n::tr!(DiffHeaderCopied)));
            assert!(text.contains("98%") && text.contains("10%"));
            assert!(!text.contains("0123456") && !text.contains("diff --git"));
        }
    }
    #[test]
    fn creation_deletion_and_type_changes_keep_their_meaning_without_code_hunks() {
        let _language = i18n::using(i18n::Language::Chinese);
        for (source, expected) in [
            (vec!["new file mode 100644"], "新增文件"),
            (vec!["deleted file mode 100755"], "删除文件"),
            (vec!["old mode 100644", "new mode 120000"], "符号链接"),
            (vec!["new file mode 160000"], "子模块"),
        ] {
            let facts = facts(&source.into_iter().map(str::to_owned).collect::<Vec<_>>()).unwrap();
            assert!(facts.iter().any(|fact| fact.text().contains(expected)));
        }
    }
}
