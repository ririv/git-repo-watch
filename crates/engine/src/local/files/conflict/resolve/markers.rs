//! Parse Git marker regions without normalizing any source bytes.
use crate::cancel::Cancel;
use anyhow::{Result, ensure};
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct Block {
    pub lines: Range<usize>,
    pub left_lines: Range<usize>,
    pub right_lines: Range<usize>,
    pub(super) whole: Range<usize>,
    pub(super) left: Range<usize>,
    pub(super) right: Range<usize>,
}
enum State {
    Outside,
    Left {
        start: usize,
        line: usize,
        content: usize,
    },
    Base {
        start: usize,
        line: usize,
        left: Range<usize>,
        left_lines: Range<usize>,
    },
    Right {
        start: usize,
        line: usize,
        left: Range<usize>,
        left_lines: Range<usize>,
        content: usize,
        content_line: usize,
    },
}
#[derive(Clone, Copy)]
enum Marker {
    Start,
    Base,
    Separator,
    End,
    Text,
}
fn marker(line: &str, size: usize) -> Marker {
    let line = line.trim_end_matches('\n').trim_end_matches('\r');
    let Some(first) = line.as_bytes().first().copied() else {
        return Marker::Text;
    };
    if !matches!(first, b'<' | b'|' | b'=' | b'>') {
        return Marker::Text;
    }
    let count = line.bytes().take_while(|b| *b == first).count();
    if count != size
        || !(line.len() == size || (first != b'=' && line.as_bytes().get(size) == Some(&b' ')))
    {
        return Marker::Text;
    }
    match first {
        b'<' => Marker::Start,
        b'|' => Marker::Base,
        b'=' => Marker::Separator,
        b'>' => Marker::End,
        _ => unreachable!(),
    }
}
pub(super) fn parse(text: &str, size: usize, cancel: &Cancel) -> Result<Vec<Block>> {
    let mut state = State::Outside;
    let mut blocks = Vec::new();
    let mut offset = 0;
    for (row, line) in text.split_inclusive('\n').enumerate() {
        cancel.check()?;
        let end = offset + line.len();
        state = match (state, marker(line, size)) {
            (State::Outside, Marker::Start) => State::Left {
                start: offset,
                line: row,
                content: end,
            },
            (State::Outside, _) => State::Outside,
            (
                State::Left {
                    start,
                    line: first,
                    content,
                },
                Marker::Base,
            ) => State::Base {
                start,
                line: first,
                left: content..offset,
                left_lines: first + 1..row,
            },
            (
                State::Left {
                    start,
                    line: first,
                    content,
                },
                Marker::Separator,
            ) => State::Right {
                start,
                line: first,
                left: content..offset,
                left_lines: first + 1..row,
                content: end,
                content_line: row + 1,
            },
            (
                State::Base {
                    start,
                    line: first,
                    left,
                    left_lines,
                },
                Marker::Separator,
            ) => State::Right {
                start,
                line: first,
                left,
                left_lines,
                content: end,
                content_line: row + 1,
            },
            (
                State::Right {
                    start,
                    line: first,
                    left,
                    left_lines,
                    content,
                    content_line,
                },
                Marker::End,
            ) => {
                blocks.push(Block {
                    whole: start..end,
                    lines: first..row + 1,
                    left,
                    left_lines,
                    right: content..offset,
                    right_lines: content_line..row,
                });
                State::Outside
            }
            (current, Marker::Text) => current,
            _ => anyhow::bail!(i18n::tr!(ConflictMarkersInvalid)),
        };
        offset = end;
    }
    ensure!(
        matches!(state, State::Outside),
        i18n::tr!(ConflictMarkersInvalid)
    );
    Ok(blocks)
}
