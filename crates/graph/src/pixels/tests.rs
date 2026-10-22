use super::*;
use crate::Layout;
fn metrics() -> Metrics {
    Metrics {
        width: NonZeroU16::new(20).unwrap(),
        height: NonZeroU16::new(40).unwrap(),
    }
}
fn alpha(p: &Pixels, x: u32, y: u32) -> u8 {
    p.rgba[((y * p.width + x) * 4 + 3) as usize]
}
#[test]
fn linear_commits_have_continuous_pixels_between_their_centers() {
    let mut layout = Layout::default();
    let a = layout.push(&3, &[2]);
    let b = layout.push(&2, &[1]);
    let c = layout.push(&1, &[]);
    let crop = Crop {
        columns: NonZeroU16::new(1).unwrap(),
        pan: 0,
    };
    let first = row(None, &a, Node::Commit, metrics(), crop).unwrap();
    let middle = row(Some(&a), &b, Node::Commit, metrics(), crop).unwrap();
    let last = row(Some(&b), &c, Node::Commit, metrics(), crop).unwrap();
    assert!((20..40).all(|y| alpha(&first, 10, y) > 0));
    assert!((0..40).all(|y| alpha(&middle, 10, y) > 0));
    assert!((0..20).all(|y| alpha(&last, 10, y) > 0));
    assert_eq!(alpha(&first, 10, 0), 0);
    assert_eq!(alpha(&last, 10, 39), 0);
}
#[test]
fn merge_curves_share_pixels_across_rows_and_pan_is_an_exact_crop() {
    let mut layout = Layout::default();
    let a = layout.push(&4, &[2, 3]);
    let b = layout.push(&2, &[1]);
    let crop = Crop {
        columns: NonZeroU16::new(3).unwrap(),
        pan: 0,
    };
    let first = row(None, &a, Node::Merge, metrics(), crop).unwrap();
    let second = row(Some(&a), &b, Node::Commit, metrics(), crop).unwrap();
    assert!((0..60).any(|x| alpha(&first, x, 39) > 0 && alpha(&second, x, 0) > 0));
    assert!(first.rgba.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255));
    let shifted = row(
        None,
        &a,
        Node::Merge,
        metrics(),
        Crop {
            columns: NonZeroU16::new(2).unwrap(),
            pan: 1,
        },
    )
    .unwrap();
    for y in 0..40 {
        for x in 0..40 {
            let expected = ((y * 60 + x + 20) * 4) as usize;
            let actual = ((y * 40 + x) * 4) as usize;
            assert_eq!(
                &first.rgba[expected..expected + 4],
                &shifted.rgba[actual..actual + 4]
            );
        }
    }
}
#[test]
fn shallow_boundary_has_a_hollow_center_and_no_invented_parent_line() {
    let mut layout = Layout::default();
    let a = layout.push(&2, &[1]);
    let b = layout.push(&1, &[]);
    let tile = row(
        Some(&a),
        &b,
        Node::Boundary,
        metrics(),
        Crop {
            columns: NonZeroU16::new(1).unwrap(),
            pan: 0,
        },
    )
    .unwrap();
    assert_eq!(alpha(&tile, 10, 20), 0);
    assert!(alpha(&tile, 10, 12) > 0);
    assert_eq!(alpha(&tile, 10, 39), 0);
}
