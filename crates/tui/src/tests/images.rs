use super::*;
use crate::{document, preview};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
fn image_bytes(color: [u8; 4], format: ImageFormat) -> Vec<u8> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    let pixels = RgbaImage::from_pixel(8, 4, Rgba(color));
    let image = DynamicImage::ImageRgba8(pixels);
    if format == ImageFormat::Jpeg {
        image.to_rgb8().write_to(&mut bytes, format).unwrap();
    } else {
        image.write_to(&mut bytes, format).unwrap();
    }
    bytes.into_inner()
}
fn images(s: &Screen) -> &preview::View {
    let content = match &s.mode {
        Mode::Changes(session) => session.content(),
        Mode::History(view) => match view.as_ref() {
            crate::history::View::Diff { content, .. } => content.as_ref(),
            _ => panic!("historical review expected"),
        },
        _ => panic!("review expected"),
    };
    let document::View::Images(images) = content else {
        panic!("image review expected")
    };
    images
}
fn pixel(side: &preview::Side) -> [u8; 4] {
    let preview::Side::Image { pixels, .. } = side else {
        panic!("decoded image expected")
    };
    pixels.get_pixel(0, 0).0
}
#[test]
fn working_and_staged_images_decode_exact_snapshots_render_pixels_and_cleanup_on_return() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let path = f.repo.join("中文 image.png");
        std::fs::write(&path, image_bytes([255, 0, 0, 255], ImageFormat::Png)).unwrap();
        support::git(&f.repo, &["add", "."]);
        support::git(&f.repo, &["commit", "-m", "image"]);
        std::fs::write(&path, image_bytes([0, 255, 0, 255], ImageFormat::Png)).unwrap();
        support::git(&f.repo, &["add", "."]);
        std::fs::write(&path, image_bytes([0, 0, 255, 255], ImageFormat::Png)).unwrap();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let refs = support::git(&f.repo, &["show-ref"]);
        open_files(&mut s);
        wait(&mut s);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert_eq!(
            (
                pixel(&images(&s).pairs[0].before),
                pixel(&images(&s).pairs[0].after)
            ),
            ([0, 255, 0, 255], [0, 0, 255, 255])
        );
        for (width, height) in [(160, 32), (60, 18), (120, 28)] {
            let text = render(&mut s, width, height);
            assert!(text.contains(i18n::tr!(ImagePreview)), "{text}");
            assert!(text.contains('\u{10eeee}'), "{text}");
            assert!(text.contains("8×4"));
        }
        key(&mut s, KeyCode::Esc);
        assert!(matches!(s.mode, Mode::Files(_)));
        render(&mut s, 120, 28);
        let Mode::Files(files) = &mut s.mode else {
            panic!("file list")
        };
        files.switch_group(s.file_layout);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert_eq!(
            (
                pixel(&images(&s).pairs[0].before),
                pixel(&images(&s).pairs[0].after)
            ),
            ([255, 0, 0, 255], [0, 255, 0, 255])
        );
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert_eq!(support::git(&f.repo, &["show-ref"]), refs);
        assert!(s.service.store.report().unwrap().is_none());
    }
}
#[test]
fn supported_formats_use_content_detection_and_history_never_reads_current_image_files() {
    for format in [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::WebP] {
        let f = support::Fixture::create();
        let path = f.repo.join("image-without-extension");
        std::fs::write(&path, image_bytes([255, 0, 0, 255], format)).unwrap();
        support::git(&f.repo, &["add", "."]);
        support::git(&f.repo, &["commit", "-m", "before"]);
        std::fs::write(&path, image_bytes([0, 255, 0, 255], format)).unwrap();
        support::git(&f.repo, &["add", "."]);
        support::git(&f.repo, &["commit", "-m", "after"]);
        std::fs::write(&path, image_bytes([0, 0, 255, 255], format)).unwrap();
        support::git(&f.repo, &["add", "."]);
        std::fs::write(&path, image_bytes([255, 255, 0, 255], format)).unwrap();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo log");
        wait(&mut s);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        let old = pixel(&images(&s).pairs[0].before);
        let new = pixel(&images(&s).pairs[0].after);
        assert!(old[0] > 240 && old[1] < 10 && new[1] > 240 && new[2] < 10);
        assert!(render(&mut s, 140, 32).contains('\u{10eeee}'));
        key(&mut s, KeyCode::Esc);
        assert!(
            matches!(&s.mode,Mode::History(view) if matches!(view.as_ref(),crate::history::View::Commit(_)))
        );
    }
}
#[test]
fn added_deleted_corrupt_and_unsupported_terminal_states_are_explicit() {
    let f = support::Fixture::create();
    let path = f.repo.join("new.png");
    std::fs::write(&path, image_bytes([255, 0, 0, 128], ImageFormat::Png)).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    open_files(&mut s);
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(matches!(images(&s).pairs[0].before, preview::Side::Absent));
    assert!(render(&mut s, 140, 32).contains(i18n::tr!(ImageAbsent)));
    s.graphics = graphics::Display::new(graphics::Support::Unavailable(
        "pixel backend unavailable".into(),
    ));
    assert!(render(&mut s, 140, 32).contains("pixel backend unavailable"));
    key(&mut s, KeyCode::Esc);
    support::git(&f.repo, &["add", "."]);
    support::git(&f.repo, &["commit", "-m", "added"]);
    std::fs::write(&path, b"\x89PNG\r\n\x1a\n\0broken").unwrap();
    s.key(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(matches!(
        images(&s).pairs[0].after,
        preview::Side::Failed(_)
    ));
    assert!(render(&mut s, 140, 32).contains(i18n::tr!(ImageDecodeFailed)));
    key(&mut s, KeyCode::Esc);
    std::fs::remove_file(path).unwrap();
    s.key(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(matches!(images(&s).pairs[0].after, preview::Side::Absent));
}
