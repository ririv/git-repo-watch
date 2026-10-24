//! Decode bounded source snapshots once; terminal-sized rasters are prepared by the graphics worker.
mod raster;
use anyhow::{Context, Result, ensure};
use engine::{
    cancel::Cancel,
    local::files::{
        media,
        patch::{FilePatch, Source},
    },
};
use image::{ImageDecoder, ImageReader, RgbaImage};
pub(super) use raster::fit;
use std::{
    io::Cursor,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
static NEXT_SOURCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub(super) enum Side {
    Absent,
    Other,
    Image {
        path: std::path::PathBuf,
        format: media::Format,
        pixels: Arc<RgbaImage>,
    },
    Failed(String),
}
#[derive(Debug)]
pub(super) struct Pair {
    pub before: Side,
    pub after: Side,
}
#[derive(Debug)]
pub(super) struct View {
    pub source: u64,
    pub title: String,
    pub labels: (String, String),
    pub pairs: Vec<Pair>,
}
pub(super) fn prepare(
    title: &str,
    files: Vec<FilePatch>,
    labels: (String, String),
    cancel: &Cancel,
) -> Result<View> {
    let mut pairs = Vec::new();
    for file in files {
        cancel.check()?;
        pairs.push(Pair {
            before: side(file.before, cancel)?,
            after: side(file.after, cancel)?,
        });
    }
    Ok(View {
        source: NEXT_SOURCE.fetch_add(1, Ordering::Relaxed),
        title: title.into(),
        labels,
        pairs,
    })
}
pub(super) fn single(image: media::Image, cancel: &Cancel) -> Result<View> {
    let title = image.path.to_string_lossy().into_owned();
    cancel.check()?;
    let after = side(Source::Image(image), cancel)?;
    Ok(View {
        source: NEXT_SOURCE.fetch_add(1, Ordering::Relaxed),
        title,
        labels: (String::new(), i18n::tr!(Worktree).into()),
        pairs: vec![Pair {
            before: Side::Absent,
            after,
        }],
    })
}
fn side(source: Source, cancel: &Cancel) -> Result<Side> {
    cancel.check()?;
    let side = match source {
        Source::Absent => Side::Absent,
        Source::Image(image) => match decode(&image) {
            Ok(pixels) => Side::Image {
                path: image.path,
                format: image.format,
                pixels: Arc::new(pixels),
            },
            Err(error) => Side::Failed(format!("{}: {error:#}", i18n::tr!(ImageDecodeFailed))),
        },
        Source::Text { .. } | Source::Unavailable(_) => Side::Other,
    };
    cancel.check()?;
    Ok(side)
}
fn decode(image: &media::Image) -> Result<RgbaImage> {
    ensure!(
        image.bytes.len() <= media::MAX_IMAGE_BYTES,
        i18n::tr!(ImageEncodedLimit)
    );
    let format = match image.format {
        media::Format::Png => image::ImageFormat::Png,
        media::Format::Jpeg => image::ImageFormat::Jpeg,
        media::Format::Webp => image::ImageFormat::WebP,
    };
    let reader = || {
        let mut reader = ImageReader::with_format(Cursor::new(&image.bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        reader
    };
    let (width, height) = reader().into_dimensions().map_err(image_error)?;
    ensure!(
        width > 0
            && height > 0
            && width <= 8192
            && height <= 8192
            && u64::from(width) * u64::from(height) <= 8_000_000,
        i18n::tr!(ImageDecodedLimit)
    );
    let mut decoder = reader().into_decoder().map_err(image_error)?;
    let orientation = decoder.orientation()?;
    let mut decoded =
        image::DynamicImage::from_decoder(decoder).context(i18n::tr!(ImageDecodeFailed))?;
    decoded.apply_orientation(orientation);
    Ok(decoded.into_rgba8())
}

fn image_error(error: image::ImageError) -> anyhow::Error {
    match error {
        image::ImageError::Limits(_) => {
            anyhow::Error::new(error).context(i18n::tr!(ImageDecodedLimit))
        }
        error => error.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jpeg_orientation_is_applied_before_display_sizing() {
        let pixels = image::RgbImage::from_pixel(8, 4, image::Rgb([255, 0, 0]));
        let mut output = Cursor::new(Vec::new());
        pixels
            .write_to(&mut output, image::ImageFormat::Jpeg)
            .unwrap();
        let exif = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
        let bytes = output.into_inner();
        let mut oriented = bytes[..2].to_vec();
        oriented.extend([0xff, 0xe1]);
        oriented.extend(((exif.len() + 2) as u16).to_be_bytes());
        oriented.extend(exif);
        oriented.extend(&bytes[2..]);
        let image = media::Image {
            path: "oriented.jpg".into(),
            format: media::Format::Jpeg,
            bytes: oriented,
        };
        assert_eq!(decode(&image).unwrap().dimensions(), (4, 8));
    }
    #[test]
    fn dimension_limits_reject_a_valid_encoded_image_before_full_decode() {
        let pixels = RgbaImage::new(8193, 1);
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(pixels)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let image = media::Image {
            path: "wide.png".into(),
            format: media::Format::Png,
            bytes: bytes.into_inner(),
        };
        assert!(
            decode(&image)
                .unwrap_err()
                .to_string()
                .contains(i18n::tr!(ImageDecodedLimit))
        );
    }
    #[test]
    fn fit_preserves_aspect_ratio_and_composites_alpha_over_checkerboard() {
        let pixels = RgbaImage::from_pixel(2, 1, image::Rgba([255, 0, 0, 128]));
        let fitted = fit(&pixels, 80, 80).unwrap();
        assert_eq!(fitted.dimensions(), (80, 80));
        assert_eq!(fitted.get_pixel(0, 0).0, [0, 0, 0, 0]);
        let center = fitted.get_pixel(40, 40).0;
        assert!(
            center[0] > 140 && center[1] < 30 && center[3] == 255,
            "{center:?}"
        );
        assert_eq!(fitted.get_pixel(0, 19).0, [0, 0, 0, 0]);
        assert!(fitted.get_pixel(0, 20).0[0] > 140);
        assert_eq!(fitted.get_pixel(0, 60).0, [0, 0, 0, 0]);
        let transparent = fit(&RgbaImage::new(2, 1), 80, 80).unwrap();
        assert_eq!(transparent.get_pixel(0, 0).0, [0, 0, 0, 0]);
        assert_eq!(transparent.get_pixel(0, 20).0, [42, 42, 42, 255]);
        assert_eq!(transparent.get_pixel(8, 20).0, [56, 56, 56, 255]);
    }
}
