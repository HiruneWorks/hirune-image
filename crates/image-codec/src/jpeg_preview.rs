use crate::{
    container::validate_jpeg,
    normalize::normalize_dynamic,
    source::{FrameLayout, static_facts},
};
use application::{
    error::{ProcessingError as Error, Result},
    images::{Limits, OutputRecipe, SourceFacts, SourceFormat, inside_dimensions},
};
use image::{DynamicImage, ImageDecoder, RgbaImage};
use jpeg_decoder::PixelFormat;
use std::io::Cursor;

pub(crate) struct PreviewImage {
    pub pixels: RgbaImage,
    pub source: SourceFacts,
    pub layout: FrameLayout,
}

pub(crate) fn decode(
    bytes: &[u8],
    limits: &Limits,
    outputs: &[OutputRecipe],
) -> Result<Option<PreviewImage>> {
    let has_icc = validate_jpeg(bytes)?;
    let mut header = image::codecs::jpeg::JpegDecoder::new(Cursor::new(bytes))
        .map_err(|_| Error::InvalidImage)?;
    let (width, height) = header.dimensions();
    let facts = static_facts(width, height);
    // Reduction must not admit a source that exceeds original-canvas or decoded-work limits.
    limits.preflight(&facts)?;
    if has_icc
        || header
            .icc_profile()
            .map_err(|_| Error::UnsupportedColorProfile)?
            .is_some()
    {
        return Ok(None);
    }
    let orientation = header.orientation().map_err(|_| Error::InvalidImage)?;
    drop(header);
    let swaps_axes = matches!(orientation.to_exif(), 5..=8);
    let display_dimensions = if swaps_axes {
        (height, width)
    } else {
        (width, height)
    };
    let required: Vec<_> = outputs
        .iter()
        .map(|output| {
            inside_dimensions(
                display_dimensions.0,
                display_dimensions.1,
                output.max_width,
                output.max_height,
            )
        })
        .collect();
    let Some(factor) = [8u32, 4, 2].into_iter().find(|factor| {
        let native = (width.div_ceil(*factor), height.div_ceil(*factor));
        let display = if swaps_axes {
            (native.1, native.0)
        } else {
            native
        };
        native != (width, height)
            && required
                .iter()
                .all(|&(w, h)| w <= display.0 && h <= display.1)
    }) else {
        return Ok(None);
    };

    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    decoder.set_max_decoding_buffer_size(limits.working_bytes as usize);
    decoder.read_info().map_err(|_| Error::InvalidImage)?;
    let info = decoder.info().ok_or(Error::InvalidImage)?;
    if (u32::from(info.width), u32::from(info.height)) != (width, height) {
        return Err(Error::InvalidImage);
    }
    if !matches!(info.pixel_format, PixelFormat::L8 | PixelFormat::RGB24) {
        return Ok(None);
    }
    let native = (
        width.div_ceil(factor) as u16,
        height.div_ceil(factor) as u16,
    );
    // Matching by the longer axis prevents a one-pixel side from choosing a coarser scale.
    let requested = if width >= height {
        (native.0, u16::MAX)
    } else {
        (u16::MAX, native.1)
    };
    let actual = decoder
        .scale(requested.0, requested.1)
        .map_err(|_| Error::InvalidImage)?;
    let display = if swaps_axes {
        (u32::from(actual.1), u32::from(actual.0))
    } else {
        (u32::from(actual.0), u32::from(actual.1))
    };
    if actual == (width as u16, height as u16)
        || required
            .iter()
            .any(|&(w, h)| w > display.0 || h > display.1)
    {
        return Ok(None);
    }
    let components = if info.pixel_format == PixelFormat::L8 {
        1
    } else {
        3
    };
    // Progressive scans retain full-resolution i16 coefficients. MCU padding can add
    // up to 31 samples per axis, even when reduced output has only a one-pixel side.
    let coefficients = (u64::from(width) + 31)
        .checked_mul(u64::from(height) + 31)
        .and_then(|samples| samples.checked_mul(components * 2));
    let working_reservation_bytes = coefficients
        .and_then(|bytes| bytes.checked_add(u64::from(actual.0) * u64::from(actual.1) * 20))
        .map(|bytes| bytes.max(u64::from(width) * u64::from(height) * 20))
        .ok_or(Error::ImageLimitsExceeded)?;
    if working_reservation_bytes > limits.working_bytes {
        return Ok(None);
    }
    let samples = decoder.decode().map_err(|_| Error::InvalidImage)?;
    let pixels = match info.pixel_format {
        PixelFormat::L8 => DynamicImage::ImageLuma8(
            image::GrayImage::from_raw(u32::from(actual.0), u32::from(actual.1), samples)
                .ok_or(Error::InvalidImage)?,
        ),
        PixelFormat::RGB24 => DynamicImage::ImageRgb8(
            image::RgbImage::from_raw(u32::from(actual.0), u32::from(actual.1), samples)
                .ok_or(Error::InvalidImage)?,
        ),
        _ => return Err(Error::InternalError),
    };
    let pixels = normalize_dynamic(pixels, orientation, None)?;
    Ok(Some(PreviewImage {
        pixels,
        source: SourceFacts {
            format: SourceFormat::Jpeg,
            byte_size: bytes.len() as u64,
            orientation: orientation.to_exif(),
            image: facts,
        },
        layout: FrameLayout {
            display_dimensions,
            working_reservation_bytes,
        },
    }))
}
