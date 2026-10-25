//! Choice hints use the row's cell budget without splitting option tokens.
use super::{clipped, command::ArgumentSyntax};
use unicode_width::UnicodeWidthStr;

pub(super) fn fit(argument: &ArgumentSyntax, width: usize) -> String {
    let full = argument.placeholder();
    if full.width() <= width {
        return full;
    }
    match argument {
        ArgumentSyntax::Input(_) => clipped(&full, width),
        ArgumentSyntax::Choices(values) => {
            for count in (1..values.len()).rev() {
                let hint = format!("[{}|…]", values[..count].join("|"));
                if hint.width() <= width {
                    return hint;
                }
            }
            clipped("[…]", width)
        }
    }
}
