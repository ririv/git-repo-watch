//! Captured scrollbar geometry keeps a drag on its original target across redraws.
use super::*;
use crate::scrollbar::{Axis, thumb};

#[derive(Debug, Clone)]
pub(crate) struct Grab {
    area: Rect,
    axis: Axis,
    values: Values,
    origin: u16,
    offset: usize,
    maximum: usize,
    travel: usize,
}
impl Map {
    pub fn grab(&self, point: Position) -> Option<Grab> {
        self.0.iter().rev().find_map(|region| {
            let Region::Track {
                area,
                axis,
                values,
                offset,
            } = region
            else {
                return None;
            };
            if !area.contains(point) {
                return None;
            }
            let (origin, start, length) = match axis {
                Axis::Horizontal => (point.x, area.x, area.width),
                Axis::Vertical => (point.y, area.y, area.height),
            };
            let length = usize::from(length);
            let maximum = match values {
                Values::Rows(_, rows) => rows.len().saturating_sub(length),
                Values::Offset(_, maximum) => *maximum,
            };
            let slider = thumb(length, *offset, maximum);
            if !slider.contains(&usize::from(origin - start)) {
                return None;
            }
            Some(Grab {
                area: *area,
                axis: *axis,
                values: values.clone(),
                origin,
                offset: (*offset).min(maximum),
                maximum,
                travel: length.saturating_sub(slider.len()),
            })
        })
    }
}
impl Grab {
    pub fn valid(&self, map: &Map) -> bool {
        map.0.iter().any(|region| matches!(region,
            Region::Track { area, axis, values, .. } if *area == self.area && *axis == self.axis && *values == self.values))
    }
    pub fn action(&self, point: Position) -> Option<Action> {
        let position = match self.axis {
            Axis::Horizontal => point.x,
            Axis::Vertical => point.y,
        };
        let delta = position as isize - self.origin as isize;
        if self.travel == 0 {
            return None;
        }
        let movement =
            (delta.unsigned_abs().saturating_mul(self.maximum) + self.travel / 2) / self.travel;
        let offset = if delta < 0 {
            self.offset.saturating_sub(movement)
        } else {
            self.offset.saturating_add(movement).min(self.maximum)
        };
        Some(match &self.values {
            Values::Offset(target, _) => Action::Scroll(*target, offset),
            Values::Rows(target, rows) => Action::DragRows {
                target: *target,
                index: rows[offset.min(rows.len() - 1)],
                offset,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_thumb_preserves_grip_and_clamps_outside_track() {
        let mut map = Map::default();
        let area = Rect::new(10, 8, 30, 1);
        map.track(
            area,
            Axis::Horizontal,
            100,
            40,
            ScrollTarget::DiffHorizontal,
        );
        let range = thumb(30, 40, 100);
        for cell in range {
            let origin = Position::new(10 + cell as u16, 8);
            let grab = map.grab(origin).unwrap();
            assert_eq!(
                grab.action(origin),
                Some(Action::Scroll(ScrollTarget::DiffHorizontal, 40))
            );
            assert_eq!(
                grab.action(Position::new(0, 0)),
                Some(Action::Scroll(ScrollTarget::DiffHorizontal, 0))
            );
            assert_eq!(
                grab.action(Position::new(200, 50)),
                Some(Action::Scroll(ScrollTarget::DiffHorizontal, 100))
            );
            assert!(grab.valid(&map));
            map.clear();
            map.track(
                area,
                Axis::Horizontal,
                100,
                90,
                ScrollTarget::DiffHorizontal,
            );
            assert!(grab.valid(&map));
            map.clear();
            map.track(
                area,
                Axis::Horizontal,
                100,
                40,
                ScrollTarget::DiffHorizontal,
            );
        }
        assert!(map.grab(Position::new(10, 8)).is_none());
        let grab = map.grab(Position::new(20, 8)).unwrap();
        map.clear();
        map.track(
            Rect::new(10, 8, 20, 1),
            Axis::Horizontal,
            100,
            40,
            ScrollTarget::DiffHorizontal,
        );
        assert!(!grab.valid(&map));
    }

    #[test]
    fn list_drag_uses_filtered_row_identity_and_rejects_changed_rows() {
        let mut map = Map::default();
        let panel = Rect::new(0, 0, 20, 12);
        let content = Rect::new(1, 1, 18, 10);
        let rows = (0..50).map(|n| n * 2).collect::<Vec<_>>();
        map.list(panel, content, &rows, 0, Target::Files);
        let grab = map.grab(Position::new(19, 1)).unwrap();
        assert_eq!(
            grab.action(Position::new(0, 100)),
            Some(Action::DragRows {
                target: Target::Files,
                index: 80,
                offset: 40
            })
        );
        assert_eq!(
            grab.action(Position::new(0, 1)),
            Some(Action::DragRows {
                target: Target::Files,
                index: 0,
                offset: 0
            })
        );
        map.clear();
        map.list(panel, content, &rows[1..], 0, Target::Files);
        assert!(!grab.valid(&map));
    }
}
