//! Classify editable command ranges using the same argument boundary as execution.
use super::{ArgumentAction, Input, argument_input, scan};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Command,
    Argument,
    Option,
}
#[derive(Debug)]
pub(crate) struct Segment {
    pub range: Range<usize>,
    pub role: Role,
}

pub(crate) fn segments(input: &Input) -> Vec<Segment> {
    let text = &input.editor().text;
    let query = input.query();
    let flag = super::named::flag_span(&query).and_then(|range| {
        let start = text.len().checked_sub(query.len() - range.start)?;
        Some(start..start + range.len())
    });
    let Some((argument, value)) = argument_input(&query) else {
        if let Some(flag) = flag {
            return vec![
                Segment {
                    range: 0..flag.start,
                    role: Role::Command,
                },
                Segment {
                    range: flag.clone(),
                    role: Role::Option,
                },
                Segment {
                    range: flag.end..text.len(),
                    role: Role::Command,
                },
            ];
        }
        return vec![Segment {
            range: 0..text.len(),
            role: Role::Command,
        }];
    };
    let start = text.len() - value.len();
    let mut result = if let Some(flag) = flag {
        vec![
            Segment {
                range: 0..flag.start,
                role: Role::Command,
            },
            Segment {
                range: flag.clone(),
                role: Role::Option,
            },
            Segment {
                range: flag.end..start,
                role: Role::Command,
            },
        ]
    } else {
        vec![Segment {
            range: 0..start,
            role: Role::Command,
        }]
    };
    let mut next = start;
    if argument == ArgumentAction::Scan {
        // 扫描沿用原有带引号的路径分词；路径内的空格和 -- 不拆成选项。
        for token in scan::tokens(value)
            .into_iter()
            .skip(1)
            .filter(|token| token.value.starts_with("--"))
        {
            let range = start + token.range.start..start + token.range.end;
            result.push(Segment {
                range: next..range.start,
                role: Role::Argument,
            });
            next = range.end;
            result.push(Segment {
                range,
                role: Role::Option,
            });
        }
    }
    result.push(Segment {
        range: next..text.len(),
        role: Role::Argument,
    });
    result
}
