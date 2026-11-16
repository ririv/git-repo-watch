//! Invalidate colored input surfaces together with their blank boundaries.
use crate::confirmation::Choice;
use ratatui::{Frame, buffer::CellDiffOption, layout::Rect};

/// Track the colored surface that was rendered in the previous frame.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum Surface {
    #[default]
    Plain,
    Actions {
        area: Rect,
        selected: Option<(crate::pointer::Hit, &'static str)>,
    },
    Prefix(Rect),
    Conflict {
        area: Rect,
        selected: crate::conflict::controls::Action,
    },
    Recovery {
        area: Rect,
        selected: engine::operations::replay::Control,
    },
    Confirmation {
        area: Rect,
        choice: Choice,
    },
}
impl Surface {
    fn area(&self) -> Option<Rect> {
        match *self {
            Self::Plain => None,
            Self::Actions { area, .. }
            | Self::Conflict { area, .. }
            | Self::Prefix(area)
            | Self::Recovery { area, .. }
            | Self::Confirmation { area, .. } => Some(area),
        }
    }
    pub fn update(&mut self, frame: &mut Frame, next: Self) {
        if *self == next {
            return;
        }
        let previous = std::mem::replace(self, next);
        // 前缀边界和按钮分隔空格在缓存里可能始终是默认背景，增量绘制会跳过它们。
        // 内容或选项变化时，同时重绘旧区域、新区域及空隙，清除终端残留的选中色块。
        for surface in [&previous, &*self] {
            if let Some(area) = surface.area() {
                let area = area.intersection(frame.area());
                for y in area.y..area.bottom() {
                    for x in area.x..area.right() {
                        frame.buffer_mut()[(x, y)].set_diff_option(CellDiffOption::AlwaysUpdate);
                    }
                }
            }
        }
    }
}
