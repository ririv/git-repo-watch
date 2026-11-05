//! Pairwise source coordinates reuse the same bounded alignment as the conflict overview.
use super::super::patch::Source;
use super::{Comparison, alignment};
use crate::cancel::Cancel;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pair {
    BaseCurrent,
    BaseIncoming,
    BaseResult,
    CurrentIncoming,
    CurrentResult,
    ResultIncoming,
}
impl Pair {
    pub const ALL: [Self; 6] = [
        Self::BaseCurrent,
        Self::BaseIncoming,
        Self::BaseResult,
        Self::CurrentIncoming,
        Self::CurrentResult,
        Self::ResultIncoming,
    ];
    pub fn index(self) -> usize {
        match self {
            Self::BaseCurrent => 0,
            Self::BaseIncoming => 1,
            Self::BaseResult => 2,
            Self::CurrentIncoming => 3,
            Self::CurrentResult => 4,
            Self::ResultIncoming => 5,
        }
    }
    pub fn indices(self) -> &'static [usize; 2] {
        match self {
            Self::BaseCurrent => &[0, 1],
            Self::BaseIncoming => &[0, 2],
            Self::BaseResult => &[0, 3],
            Self::CurrentIncoming => &[1, 2],
            Self::CurrentResult => &[1, 3],
            Self::ResultIncoming => &[3, 2],
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::BaseCurrent => i18n::tr!(ConflictCompareCurrent),
            Self::BaseIncoming => i18n::tr!(ConflictCompareIncoming),
            Self::BaseResult => i18n::tr!(ConflictCompareResult),
            Self::CurrentIncoming => i18n::tr!(ConflictCompareSides),
            Self::CurrentResult => i18n::tr!(ConflictCompareLeftResult),
            Self::ResultIncoming => i18n::tr!(ConflictCompareResultRight),
        }
    }
}
pub fn compare(before: &Source, after: &Source, cancel: &Cancel) -> anyhow::Result<Comparison> {
    alignment::between(before, after, cancel)
}
#[cfg(test)]
mod tests {
    use super::super::Cell;
    use super::*;
    #[test]
    fn pair_alignment_preserves_insertions_deletions_and_absent_versions() {
        let text = |text: &str| Source::Text {
            path: "中文 空格.ts".into(),
            text: text.into(),
        };
        let Comparison::Text(rows) = compare(
            &text("a\nb\nc\n"),
            &text("a\ninsert\nb\nc\n"),
            &Cancel::default(),
        )
        .unwrap() else {
            panic!()
        };
        assert_eq!(
            (rows[1].base, rows[1].current),
            (Cell::Empty, Cell::Changed(1))
        );
        assert_eq!(
            (rows[3].base, rows[3].current),
            (Cell::Context(2), Cell::Context(3))
        );
        let Comparison::Text(rows) =
            compare(&Source::Absent, &text("new\n"), &Cancel::default()).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            (rows[0].base, rows[0].current),
            (Cell::Empty, Cell::Changed(0))
        );
        assert!(matches!(
            compare(
                &text("x\n".repeat(50_001).as_str()),
                &text("x"),
                &Cancel::default()
            )
            .unwrap(),
            Comparison::LineLimit
        ));
        assert!(matches!(
            compare(
                &text("x"),
                &Source::Unavailable(super::super::super::patch::SourceOmission::Binary),
                &Cancel::default()
            )
            .unwrap(),
            Comparison::NonText
        ));
    }
}
