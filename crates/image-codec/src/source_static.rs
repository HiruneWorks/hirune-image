use crate::{
    container::validate_jpeg,
    normalize::{normalize_dynamic, profile},
    source::{Description, Emit, decoder_limits, static_facts},
};
use application::{
    error::{ProcessingError as Error, Result},
    images::{Limits, SourceFormat},
};
use image::{DynamicImage, ImageDecoder, ImageFormat, metadata::Orientation};
use std::io::Cursor;

fn image<D: ImageDecoder>(
    mut decoder: D,
    format: SourceFormat,
    limits: &Limits,
    emit: &mut Emit<'_>,
) -> Result<Description> {
    let (width, height) = decoder.dimensions();
    if matches!(
        decoder.color_type(),
        image::ColorType::Rgb32F | image::ColorType::Rgba32F
    ) {
        return Err(Error::UnsupportedColorProfile);
    }
    let facts = static_facts(width, height);
    limits.preflight(&facts)?;
    decoder
        .set_limits(decoder_limits(limits))
        .map_err(|_| Error::ImageLimitsExceeded)?;
    let icc = decoder
        .icc_profile()
        .map_err(|_| Error::UnsupportedColorProfile)?;
    let orientation = decoder.orientation().map_err(|_| Error::InvalidImage)?;
    let pixels = DynamicImage::from_decoder(decoder).map_err(|_| Error::InvalidImage)?;
    emit(
        normalize_dynamic(pixels, orientation, icc.as_deref())?,
        0,
        Orientation::NoTransforms,
        None,
    )?;
    Ok((format, facts, orientation.to_exif()))
}

pub(crate) fn jpeg(bytes: &[u8], limits: &Limits, emit: &mut Emit<'_>) -> Result<Description> {
    let has_icc = validate_jpeg(bytes)?;
    let mut decoder = image::codecs::jpeg::JpegDecoder::new(Cursor::new(bytes))
        .map_err(|_| Error::InvalidImage)?;
    let (width, height) = decoder.dimensions();
    let facts = static_facts(width, height);
    limits.preflight(&facts)?;
    let icc = decoder
        .icc_profile()
        .map_err(|_| Error::UnsupportedColorProfile)?;
    if has_icc && icc.is_none() {
        return Err(Error::UnsupportedColorProfile);
    }
    if let Some(icc) = icc.as_deref() {
        let source_profile = profile(icc)?;
        if source_profile.color_space == moxcms::DataColorSpace::Cmyk {
            use zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};
            let orientation = decoder.orientation().map_err(|_| Error::InvalidImage)?;
            drop(decoder);
            let options = DecoderOptions::default()
                .set_max_width(limits.input_dimension as usize)
                .set_max_height(limits.input_dimension as usize)
                .set_strict_mode(false)
                .jpeg_set_out_colorspace(ColorSpace::CMYK);
            let mut decoder =
                zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(bytes), options);
            decoder.decode_headers().map_err(|_| Error::InvalidImage)?;
            if !matches!(
                decoder.input_colorspace(),
                Some(ColorSpace::CMYK | ColorSpace::YCCK)
            ) {
                return Err(Error::UnsupportedColorProfile);
            }
            let mut samples = decoder.decode().map_err(|_| Error::InvalidImage)?;
            // Adobe JPEG stores inverted CMYK samples. ICC transforms consume conventional ink amounts.
            if adobe_inverted(bytes)? {
                for sample in &mut samples {
                    *sample = 255 - *sample;
                }
            }
            let transform = source_profile
                .create_transform_8bit(
                    moxcms::Layout::Rgba,
                    &moxcms::ColorProfile::new_srgb(),
                    moxcms::Layout::Rgb,
                    Default::default(),
                )
                .map_err(|_| Error::UnsupportedColorProfile)?;
            let mut rgb = vec![0; (u64::from(width) * u64::from(height) * 3) as usize];
            transform
                .transform(&samples, &mut rgb)
                .map_err(|_| Error::UnsupportedColorProfile)?;
            let pixels =
                image::RgbImage::from_raw(width, height, rgb).ok_or(Error::InvalidImage)?;
            emit(
                normalize_dynamic(DynamicImage::ImageRgb8(pixels), orientation, None)?,
                0,
                Orientation::NoTransforms,
                None,
            )?;
            return Ok((SourceFormat::Jpeg, facts, orientation.to_exif()));
        }
    }
    image(decoder, SourceFormat::Jpeg, limits, emit)
}

pub(crate) fn decode(bytes: &[u8], limits: &Limits, emit: &mut Emit<'_>) -> Result<Description> {
    let detected = image::guess_format(bytes).map_err(|_| Error::UnsupportedFormat)?;
    let format = match detected {
        ImageFormat::Bmp => SourceFormat::Bmp,
        ImageFormat::Ico => SourceFormat::Ico,
        ImageFormat::Qoi => SourceFormat::Qoi,
        ImageFormat::Pnm => SourceFormat::Pnm,
        ImageFormat::Tiff => {
            let decoder =
                tiff::decoder::Decoder::new(Cursor::new(bytes)).map_err(|_| Error::InvalidImage)?;
            if decoder.more_images() {
                return Err(Error::UnsupportedFormat);
            }
            SourceFormat::Tiff
        }
        _ => return Err(Error::UnsupportedFormat),
    };
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), detected);
    reader.limits(decoder_limits(limits));
    let decoder = reader.into_decoder().map_err(|_| Error::InvalidImage)?;
    image(decoder, format, limits, emit)
}

fn adobe_inverted(bytes: &[u8]) -> Result<bool> {
    let mut cursor = 2;
    while cursor < bytes.len() {
        if bytes.get(cursor) != Some(&255) {
            return Err(Error::InvalidImage);
        }
        while bytes.get(cursor) == Some(&255) {
            cursor += 1;
        }
        let marker = *bytes.get(cursor).ok_or(Error::InvalidImage)?;
        cursor += 1;
        if marker == 0xda {
            return Ok(false);
        }
        let length = bytes.get(cursor..cursor + 2).ok_or(Error::InvalidImage)?;
        let length = usize::from(u16::from_be_bytes([length[0], length[1]]));
        if length < 2 {
            return Err(Error::InvalidImage);
        }
        let segment = bytes
            .get(cursor + 2..cursor + length)
            .ok_or(Error::InvalidImage)?;
        if marker == 0xee && segment.starts_with(b"Adobe") {
            return Ok(true);
        }
        cursor += length;
    }
    Err(Error::InvalidImage)
}
