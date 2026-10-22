use super::*;
use ratatui::{Terminal, backend::TestBackend};
use std::{
    sync::Arc,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
fn key(index: usize) -> Key {
    Key {
        source: 1,
        index,
        pan: 0,
        columns: 2,
        metrics: Backend::test().metrics,
    }
}
fn ready(display: &mut Display) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while display
        .cache
        .values()
        .any(|entry| matches!(entry.tile, Tile::Loading(_)))
    {
        assert!(Instant::now() < deadline);
        display.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
}
#[test]
fn rgba_tiles_are_cached_rendered_as_real_kitty_pixels_and_retired_on_leave() {
    let mut display = Display::new(Support::Pixels(Backend::test()));
    let count = Arc::new(AtomicUsize::new(0));
    display.begin_frame();
    display.pin([key(0)]);
    for _ in 0..2 {
        let count = count.clone();
        display.request(key(0), move || {
            count.fetch_add(1, Ordering::Relaxed);
            Ok(RgbaImage::from_pixel(20, 20, image::Rgba([255, 0, 0, 255])))
        });
    }
    ready(&mut display);
    assert_eq!(count.load(Ordering::Relaxed), 1);
    let mut terminal = Terminal::new(TestBackend::new(20, 10)).unwrap();
    terminal
        .draw(|frame| {
            assert!(matches!(
                display.render(frame, key(0), Rect::new(2, 2, 2, 1)),
                Draw::Ready
            ));
        })
        .unwrap();
    let symbol = terminal.backend().buffer()[(2, 2)].symbol();
    assert!(symbol.contains("f=32") && symbol.contains("\u{10eeee}"));
    assert!(!symbol.contains("f=100"));
    let mut output = Vec::new();
    display.finish_frame(&mut output).unwrap();
    assert!(output.is_empty());
    display.begin_frame();
    display.finish_frame(&mut output).unwrap();
    assert!(String::from_utf8(output).unwrap().contains("a=d,d=I"));
    assert!(display.cache.is_empty());
}
#[test]
fn cache_and_pending_work_stay_bounded_as_viewports_change() {
    let mut display = Display::new(Support::Pixels(Backend::test()));
    for index in 0..300 {
        display.begin_frame();
        display.pin([key(index)]);
        display.request(key(index), || Ok(RgbaImage::new(20, 20)));
        display.poll();
    }
    ready(&mut display);
    assert!(display.cache.len() <= CAPACITY);
    assert!(display.cache.contains_key(&key(299)));
    assert!(!display.retired.is_empty());
}
