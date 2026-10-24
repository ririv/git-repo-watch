//! Static SVG parsing and viewport-sized rasterization reuse the image surface.
use anyhow::{Context, Result, ensure};
use engine::cancel::Cancel;
use image::RgbaImage;
use resvg::{tiny_skia, usvg};
use std::{
    path::Path,
    sync::{Arc, Mutex, OnceLock},
};

static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
pub(crate) fn prepare(path: &Path, source: &str, cancel: &Cancel) -> Result<super::View> {
    cancel.check()?;
    let fonts = FONTS
        .get_or_init(|| {
            let mut fonts = usvg::fontdb::Database::new();
            fonts.load_system_fonts();
            Arc::new(fonts)
        })
        .clone();
    let external = Arc::new(Mutex::new(Vec::new()));
    let requests = external.clone();
    let options = usvg::Options {
        fontdb: fonts,
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_string: Box::new(move |href, _| {
                requests
                    .lock()
                    .expect("SVG resource requests")
                    .push(href.to_owned());
                None
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(source, &options).context(i18n::tr!(SvgInvalid))?;
    ensure!(
        external.lock().expect("SVG resource requests").is_empty(),
        i18n::tr!(SvgExternal)
    );
    cancel.check()?;
    Ok(super::single_prepared(
        path.to_string_lossy().into_owned(),
        super::Side::Vector {
            path: path.to_owned(),
            tree: Arc::new(tree),
        },
    ))
}
pub(crate) fn raster(tree: &usvg::Tree, width: u32, height: u32) -> Result<RgbaImage> {
    ensure!(
        width > 0 && height > 0 && u128::from(width) * u128::from(height) * 4 <= 32 * 1024 * 1024,
        i18n::tr!(GraphicsTooLarge)
    );
    let size = tree.size();
    let scale = (width as f32 / size.width()).min(height as f32 / size.height());
    let target_width = (size.width() * scale).floor().max(1.0) as u32;
    let target_height = (size.height() * scale).floor().max(1.0) as u32;
    let mut pixmap =
        tiny_skia::Pixmap::new(target_width, target_height).context(i18n::tr!(GraphicsTooLarge))?;
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // tiny-skia 使用预乘 alpha，图片通道使用普通 RGBA。
    let bytes = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();
    let image =
        RgbaImage::from_raw(target_width, target_height, bytes).expect("SVG pixmap dimensions");
    super::fit(&image, width, height)
}
