//! OSC 11 reports are parsed independently from optional image capabilities.
use super::Background;

#[derive(Default)]
pub(crate) struct Probe {
    parser: vte::Parser,
    background: Background,
}
impl Probe {
    pub fn push(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.background, bytes);
    }
    pub fn background(&self) -> Background {
        self.background
    }
}
impl vte::Perform for Background {
    fn osc_dispatch(&mut self, params: &[&[u8]], _: bool) {
        let [b"11", spec] = params else {
            return;
        };
        let Some((red, green, blue)) = rgb(spec) else {
            return;
        };
        *self = Self::rgb(red, green, blue);
    }
}
fn rgb(spec: &[u8]) -> Option<(u8, u8, u8)> {
    let value = std::str::from_utf8(spec).ok()?.strip_prefix("rgb:")?;
    let parts = value.split('/').collect::<Vec<_>>();
    let [red, green, blue] = parts.as_slice() else {
        return None;
    };
    let channel = |part: &str| {
        if !(1..=4).contains(&part.len()) || !part.bytes().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let value = u32::from_str_radix(part, 16).ok()?;
        let maximum = (1u32 << (4 * part.len())) - 1;
        Some(((value * 255 + maximum / 2) / maximum) as u8)
    };
    Some((channel(red)?, channel(green)?, channel(blue)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_osc_reports_accept_standard_channel_widths_and_both_terminators() {
        for (report, expected) in [
            ("\x1b]11;rgb:f/f/f\x07", Background::Light),
            ("\x1b]11;rgb:ff/ff/ff\x1b\\", Background::Light),
            ("\x1b]11;rgb:fff/fff/fff\x07", Background::Light),
            ("\x1b]11;rgb:ffff/ffff/ffff\x1b\\", Background::Light),
            ("\x1b]11;rgb:2323/2121/3636\x07", Background::Dark),
        ] {
            let mut probe = Probe::default();
            for byte in report.as_bytes() {
                probe.push(&[*byte]);
            }
            assert_eq!(probe.background(), expected);
        }
    }

    #[test]
    fn unrelated_and_malformed_reports_do_not_invent_a_background() {
        let mut probe = Probe::default();
        probe.push(b"\x1b]10;rgb:ff/ff/ff\x07\x1b]11;rgb:fffff/ff/ff\x07\x1b[0n");
        assert_eq!(probe.background(), Background::Unreported);
    }
}
