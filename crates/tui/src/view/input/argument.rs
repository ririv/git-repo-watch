//! Ghost parameter hints share command declarations without entering the editable draft.
use super::{MUTED, Span, Style, truncate};
use crate::command::{self, ArgumentSyntax, Input};
use unicode_width::UnicodeWidthStr;

fn fit(argument: &ArgumentSyntax, width: usize) -> String {
    let full = argument.placeholder();
    if full.width() <= width {
        return full;
    }
    match argument {
        ArgumentSyntax::Input(_) => truncate(&full, width),
        ArgumentSyntax::Choices(values) => {
            for count in (1..values.len()).rev() {
                let hint = format!("[{}|…]", values[..count].join("|"));
                if hint.width() <= width {
                    return hint;
                }
            }
            truncate("[…]", width)
        }
    }
}

pub(super) fn hint(
    input: &Input,
    config: &engine::state::Config,
    width: usize,
) -> Option<Span<'static>> {
    let editor = input.editor();
    if width == 0
        || input.prefix_selected()
        || editor.text.is_empty()
        || editor.cursor != editor.text.len()
    {
        return None;
    }
    let query = input.query();
    let (argument, value) = command::argument_input(&query)?;
    if !value.trim().is_empty() || !argument.action().available(config) {
        return None;
    }
    Some(Span::styled(
        fit(&argument.syntax(), width),
        Style::default().fg(MUTED).italic(),
    ))
}
