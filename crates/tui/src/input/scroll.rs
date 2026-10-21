//! Keyboard ownership cancels the remaining events in an active wheel sequence.
use std::time::{Duration, Instant};
// 复现现象：触控板滚到列表底部后立即按 ↑，选中项像是没有移动，要再按一次；
// 滚到顶部后按 ↓ 也一样。等待惯性结束或点击窗口后再按键，通常不会出现。
// 原因不是索引越界：方向键已经移动了选中项，但随后到达的同向惯性滚轮事件又把它拉回边界。
// 实际日志中，↑ 将索引从 25 改为 24，仅约 3ms 后的 ScrollDown 又将其改回 25。
// 终端鼠标协议没有手势结束或惯性阶段字段，因此用 250ms 无滚轮事件的间隔划分滚动序列；
// 这是输入交接策略，并非系统报告的惯性时长。键盘接管后忽略该序列剩余的同向事件，
// 反向滚动立即接受，停顿后的同向新手势也正常接受，避免永久锁住滚轮。
const SEQUENCE_GAP: Duration = Duration::from_millis(250);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Up,
    Down,
}
#[derive(Debug, Default)]
pub(crate) enum ScrollInput {
    #[default]
    Idle,
    Wheel {
        direction: Direction,
        last_event: Instant,
    },
    Keyboard {
        blocked_direction: Direction,
        last_event: Instant,
    },
}
impl ScrollInput {
    pub(crate) fn keyboard(&mut self, now: Instant) {
        *self = match *self {
            Self::Wheel {
                direction,
                last_event,
            }
            | Self::Keyboard {
                blocked_direction: direction,
                last_event,
            } if now.duration_since(last_event) < SEQUENCE_GAP => Self::Keyboard {
                blocked_direction: direction,
                last_event,
            },
            _ => Self::Idle,
        };
    }
    pub(crate) fn wheel(&mut self, direction: Direction, now: Instant) -> bool {
        if let Self::Keyboard {
            blocked_direction,
            last_event,
        } = self
            && *blocked_direction == direction
            && now.duration_since(*last_event) < SEQUENCE_GAP
        {
            // 被忽略的惯性事件仍会延续原序列，间隔应从最近一次滚轮事件重新计时。
            *last_event = now;
            return false;
        }
        *self = Self::Wheel {
            direction,
            last_event: now,
        };
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_trace_keyboard_then_three_millisecond_momentum_does_not_retake_selection() {
        for (direction, reverse) in [
            (Direction::Down, Direction::Up),
            (Direction::Up, Direction::Down),
        ] {
            let mut input = ScrollInput::default();
            let start = Instant::now();
            assert!(input.wheel(direction, start + Duration::from_millis(3413)));
            input.keyboard(start + Duration::from_millis(3442));
            for ms in [3445, 3497, 3559, 3626, 3729] {
                assert!(!input.wheel(direction, start + Duration::from_millis(ms)));
            }
            assert!(input.wheel(reverse, start + Duration::from_millis(3730)));
        }
    }
    #[test]
    fn a_new_same_direction_gesture_is_accepted_after_the_sequence_ends() {
        let start = Instant::now();
        let mut input = ScrollInput::default();
        assert!(input.wheel(Direction::Down, start));
        input.keyboard(start + Duration::from_millis(10));
        assert!(!input.wheel(Direction::Down, start + Duration::from_millis(20)));
        assert!(input.wheel(Direction::Down, start + Duration::from_millis(270)));
        input.keyboard(start + Duration::from_millis(600));
        assert!(input.wheel(Direction::Down, start + Duration::from_millis(601)));
    }
}
