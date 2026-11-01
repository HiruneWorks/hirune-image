use crate::{
    container::{gif_structure, webp_structure},
    normalize::normalize,
    source_avif, source_heic, source_jxl, source_png, source_static, source_svg, webp,
};
use application::{
    error::{ProcessingError as Error, Result},
    images::{Budget, ImageFacts, Limits, SourceFacts, SourceFormat},
};
use image::{AnimationDecoder, RgbaImage, metadata::Orientation};
use std::io::Cursor;

pub struct DecodedSource {
    pub facts: SourceFacts,
    pub delays: Vec<u32>,
    pub normalized_dimensions: (u32, u32),
}
pub(crate) type Emit<'a> = dyn FnMut(RgbaImage, u32, Orientation, Option<&[u8]>) -> Result<()> + 'a;
pub(crate) type Description = (SourceFormat, ImageFacts, u8);

pub(crate) fn decoder_limits(limits: &Limits) -> image::Limits {
    let mut result = image::Limits::default();
    result.max_image_width = Some(limits.input_dimension);
    result.max_image_height = Some(limits.input_dimension);
    result.max_alloc = Some(limits.working_bytes);
    result
}
pub(crate) fn static_facts(width: u32, height: u32) -> ImageFacts {
    ImageFacts {
        animated: false,
        width,
        height,
        frame_count: 1,
        duration_ms: 0,
        loop_count: 1,
    }
}
pub fn decode_source(
    bytes: &[u8],
    limits: &Limits,
    mut on_decoder_start: impl FnMut(),
    mut frame: impl FnMut(RgbaImage, u32) -> Result<()>,
) -> Result<DecodedSource> {
    limits.validate()?;
    let mut normalized_dimensions = (0, 0);
    let mut budget = Budget::default();
    let mut delays = Vec::new();
    let mut emit =
        |rgba: RgbaImage, delay: u32, orientation: Orientation, icc: Option<&[u8]>| -> Result<()> {
            budget.frame(rgba.width(), rgba.height(), delay, limits)?;
            let normalized = normalize(rgba, orientation, icc)?;
            if normalized_dimensions != (0, 0) && normalized_dimensions != normalized.dimensions() {
                return Err(Error::UnrepresentableAnimation);
            }
            normalized_dimensions = normalized.dimensions();
            frame(normalized, delay)?;
            delays.push(delay);
            Ok(())
        };
    on_decoder_start();
    let (format, facts, orientation) = if bytes.starts_with(b"GIF8") {
        let structure = gif_structure(bytes, limits)?;
        limits.preflight(&structure.facts)?;
        let mut decoder = image::codecs::gif::GifDecoder::new(Cursor::new(bytes))
            .map_err(|_| Error::InvalidImage)?;
        use image::ImageDecoder;
        decoder
            .set_limits(decoder_limits(limits))
            .map_err(|_| Error::ImageLimitsExceeded)?;
        let mut count = 0;
        for decoded in decoder.into_frames() {
            let decoded = decoded.map_err(|_| Error::InvalidImage)?;
            let expected = *structure.delays.get(count).ok_or(Error::InvalidImage)?;
            let (n, d) = decoded.delay().numer_denom_ms();
            if structure.facts.animated && (d == 0 || n % d != 0 || n / d != expected) {
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
        (SourceFormat::Gif, structure.facts, 1)
    } else if bytes.starts_with(b"RIFF") {
        let structure = webp_structure(bytes, limits)?;
        let facts = webp::decode_frames(bytes, limits, |data, delay| {
            let rgba = RgbaImage::from_raw(structure.facts.width, structure.facts.height, data)
                .ok_or(Error::InvalidImage)?;
            emit(rgba, delay, structure.orientation, structure.icc)
        })?;
        (SourceFormat::Webp, facts, structure.orientation.to_exif())
    } else if bytes.starts_with(&[255, 216]) {
        source_static::jpeg(bytes, limits, &mut emit)?
    } else if bytes.starts_with(b"\x89PNG") {
        source_png::decode(bytes, limits, &mut emit)?
    } else if bytes.starts_with(&[255, 10]) || bytes.starts_with(b"\0\0\0\x0cJXL \r\n\x87\n") {
        source_jxl::decode(bytes, limits, &mut emit)?
    } else if bytes.get(4..8) == Some(b"ftyp") {
        if source_heic::is_avif(bytes)? {
            source_avif::decode(bytes, limits, &mut emit)?
        } else {
            source_heic::decode(bytes, limits, &mut emit)?
        }
    } else if bytes
        .strip_prefix(b"\xef\xbb\xbf")
        .unwrap_or(bytes)
        .iter()
        .find(|b| !b.is_ascii_whitespace())
        == Some(&b'<')
    {
        source_svg::decode(bytes, limits, &mut emit)?
    } else {
        source_static::decode(bytes, limits, &mut emit)?
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
