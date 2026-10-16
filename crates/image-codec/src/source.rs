use crate::{
    container::{gif_structure, validate_jpeg, validate_png, webp_structure},
    normalize::normalize,
    webp,
};
use application::{
    error::{ProcessingError as Error, Result},
    images::{Budget, ImageFacts, Limits, SourceFacts, SourceFormat},
};
use image::{AnimationDecoder, DynamicImage, ImageDecoder, RgbaImage, metadata::Orientation};
use std::io::Cursor;

pub struct DecodedSource {
    pub facts: SourceFacts,
    pub delays: Vec<u32>,
    pub normalized_dimensions: (u32, u32),
}
fn decoder_limits(limits: &Limits) -> image::Limits {
    let mut result = image::Limits::default();
    result.max_image_width = Some(limits.input_dimension);
    result.max_image_height = Some(limits.input_dimension);
    result.max_alloc = Some(limits.working_bytes);
    result
}
pub fn decode_source(
    bytes: &[u8],
    limits: &Limits,
    mut on_decoder_start: impl FnMut(),
    mut frame: impl FnMut(RgbaImage, u32) -> Result<()>,
) -> Result<DecodedSource> {
    let mut normalized_dimensions = (0, 0);
    let mut budget = Budget::default();
    let mut emit =
        |rgba: RgbaImage, delay: u32, orientation: Orientation, icc: Option<&[u8]>| -> Result<()> {
            budget.frame(rgba.width(), rgba.height(), delay, limits)?;
            let normalized = normalize(rgba, orientation, icc)?;
            normalized_dimensions = normalized.dimensions();
            frame(normalized, delay)
        };
    let (format, facts, orientation, delays) = if bytes.starts_with(b"GIF8") {
        let structure = gif_structure(bytes)?;
        limits.preflight(&structure.facts)?;
        let mut decoder = image::codecs::gif::GifDecoder::new(Cursor::new(bytes))
            .map_err(|_| Error::InvalidImage)?;
        decoder
            .set_limits(decoder_limits(limits))
            .map_err(|_| Error::ImageLimitsExceeded)?;
        let mut count = 0;
        on_decoder_start();
        for decoded in decoder.into_frames() {
            let decoded = decoded.map_err(|_| Error::InvalidImage)?;
            let expected = *structure.delays.get(count).ok_or(Error::InvalidImage)?;
            let (n, d) = decoded.delay().numer_denom_ms();
            if structure.facts.frame_count > 1 && (d == 0 || n % d != 0 || n / d != expected) {
                return Err(Error::InvalidImage);
            }
            emit(
                decoded.into_buffer(),
                expected,
                Orientation::NoTransforms,
                None,
            )?;
            count += 1;
        }
        if count != structure.delays.len() {
            return Err(Error::InvalidImage);
        }
        (SourceFormat::Gif, structure.facts, 1, structure.delays)
    } else if bytes.starts_with(b"RIFF") {
        let structure = webp_structure(bytes)?;
        let orientation = structure.orientation;
        let icc = structure.icc;
        on_decoder_start();
        let facts = webp::decode_frames(bytes, limits, |data, delay| {
            let rgba = RgbaImage::from_raw(structure.facts.width, structure.facts.height, data)
                .ok_or(Error::InvalidImage)?;
            emit(rgba, delay, orientation, icc)
        })?;
        (
            SourceFormat::Webp,
            facts,
            orientation.to_exif(),
            structure.delays,
        )
    } else if bytes.starts_with(&[255, 216]) {
        let has_icc = validate_jpeg(bytes)?;
        let mut decoder = image::codecs::jpeg::JpegDecoder::new(Cursor::new(bytes))
            .map_err(|_| Error::InvalidImage)?;
        let (w, h) = decoder.dimensions();
        let facts = ImageFacts {
            width: w,
            height: h,
            frame_count: 1,
            duration_ms: 0,
            loop_count: 1,
        };
        limits.preflight(&facts)?;
        decoder
            .set_limits(decoder_limits(limits))
            .map_err(|_| Error::ImageLimitsExceeded)?;
        let icc = decoder
            .icc_profile()
            .map_err(|_| Error::UnsupportedColorProfile)?;
        if icc.is_none() && has_icc {
            return Err(Error::UnsupportedColorProfile);
        }
        let orientation = decoder.orientation().map_err(|_| Error::InvalidImage)?;
        on_decoder_start();
        let pixels = DynamicImage::from_decoder(decoder)
            .map_err(|_| Error::InvalidImage)?
            .into_rgba8();
        emit(pixels, 0, orientation, icc.as_deref())?;
        (SourceFormat::Jpeg, facts, orientation.to_exif(), vec![0])
    } else if bytes.starts_with(b"\x89PNG") {
        let has_icc = validate_png(bytes)?;
        let width = u32::from_be_bytes(bytes[16..20].try_into().map_err(|_| Error::InvalidImage)?);
        let height = u32::from_be_bytes(bytes[20..24].try_into().map_err(|_| Error::InvalidImage)?);
        limits.preflight(&ImageFacts {
            width,
            height,
            frame_count: 1,
            duration_ms: 0,
            loop_count: 1,
        })?;
        let mut metadata_limits = decoder_limits(limits);
        metadata_limits.max_alloc = Some(limits.working_bytes.min(2 << 20));
        let mut decoder =
            image::codecs::png::PngDecoder::with_limits(Cursor::new(bytes), metadata_limits)
                .map_err(|_| {
                    if has_icc {
                        Error::UnsupportedColorProfile
                    } else {
                        Error::InvalidImage
                    }
                })?;
        decoder
            .set_limits(decoder_limits(limits))
            .map_err(|_| Error::ImageLimitsExceeded)?;
        let (w, h) = decoder.dimensions();
        let facts = ImageFacts {
            width: w,
            height: h,
            frame_count: 1,
            duration_ms: 0,
            loop_count: 1,
        };
        limits.preflight(&facts)?;
        let icc = decoder
            .icc_profile()
            .map_err(|_| Error::UnsupportedColorProfile)?;
        if icc.is_none() && has_icc {
            return Err(Error::UnsupportedColorProfile);
        }
        let orientation = decoder.orientation().map_err(|_| Error::InvalidImage)?;
        on_decoder_start();
        let pixels = DynamicImage::from_decoder(decoder)
            .map_err(|_| Error::InvalidImage)?
            .into_rgba8();
        emit(pixels, 0, orientation, icc.as_deref())?;
        (SourceFormat::Png, facts, orientation.to_exif(), vec![0])
    } else {
        return Err(Error::UnsupportedFormat);
    };
    if budget.frames != facts.frame_count || budget.duration_ms != facts.duration_ms {
        return Err(Error::InvalidImage);
    }
    Ok(DecodedSource {
        facts: SourceFacts {
            format,
            byte_size: bytes.len() as u64,
            orientation,
            image: facts,
        },
        delays,
        normalized_dimensions,
    })
}
