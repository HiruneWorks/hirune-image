use crate::{
    normalize::normalize_dynamic_with_profile,
    png_container,
    source::{Description, Emit, decoder_limits},
    timeline::Timeline,
};
use application::{
    error::{ProcessingError as Error, Result},
    images::{Limits, SourceFormat},
};
use image::{DynamicImage, ImageDecoder, metadata::Orientation};
use std::io::Cursor;

pub(crate) fn decode(bytes: &[u8], limits: &Limits, emit: &mut Emit<'_>) -> Result<Description> {
    let color = png_container::inspect(bytes)?;
    let width = u32::from_be_bytes(bytes[16..20].try_into().map_err(|_| Error::InvalidImage)?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().map_err(|_| Error::InvalidImage)?);
    let mut facts = super::source::static_facts(width, height);
    let mut delays = Vec::new();
    let mut plays = None;
    let mut cursor = 8;
    let mut clock = Timeline::default();
    while cursor < bytes.len() {
        let length = u32::from_be_bytes(
            bytes[cursor..cursor + 4]
                .try_into()
                .map_err(|_| Error::InvalidImage)?,
        ) as usize;
        let body = &bytes[cursor + 8..cursor + 8 + length];
        match &bytes[cursor + 4..cursor + 8] {
            b"acTL" => {
                if plays.is_some() || length != 8 {
                    return Err(Error::InvalidImage);
                }
                facts.animated = true;
                facts.frame_count =
                    u32::from_be_bytes(body[..4].try_into().map_err(|_| Error::InvalidImage)?);
                plays = Some(u32::from_be_bytes(
                    body[4..].try_into().map_err(|_| Error::InvalidImage)?,
                ));
                if facts.frame_count == 0 || facts.frame_count > limits.frames {
                    return Err(Error::ImageLimitsExceeded);
                }
            }
            b"fcTL" => {
                if !facts.animated || length != 26 {
                    return Err(Error::InvalidImage);
                }
                if delays.len() >= limits.frames as usize {
                    return Err(Error::ImageLimitsExceeded);
                }
                let num = u16::from_be_bytes([body[20], body[21]]);
                let den = u16::from_be_bytes([body[22], body[23]]);
                let delay =
                    clock.push(u64::from(num), u64::from(if den == 0 { 100 } else { den }))?;
                facts.duration_ms = facts
                    .duration_ms
                    .checked_add(delay)
                    .ok_or(Error::ImageLimitsExceeded)?;
                if facts.duration_ms > limits.duration_ms {
                    return Err(Error::ImageLimitsExceeded);
                }
                delays.push(delay);
            }
            _ => {}
        }
        cursor += length + 12;
    }
    if facts.animated {
        if delays.len() != facts.frame_count as usize {
            return Err(Error::InvalidImage);
        }
        facts.loop_count = u16::try_from(plays.ok_or(Error::InvalidImage)?)
            .map_err(|_| Error::UnrepresentableAnimation)?;
    }
    limits.preflight(&facts)?;
    let mut metadata_limits = decoder_limits(limits);
    metadata_limits.max_alloc = Some(limits.working_bytes.min(2 << 20));
    let mut decoder =
        image::codecs::png::PngDecoder::with_limits(Cursor::new(bytes), metadata_limits).map_err(
            |_| {
                if color.has_icc {
                    Error::UnsupportedColorProfile
                } else {
                    Error::InvalidImage
                }
            },
        )?;
    decoder
        .set_limits(decoder_limits(limits))
        .map_err(|_| Error::ImageLimitsExceeded)?;
    let icc = decoder
        .icc_profile()
        .map_err(|_| Error::UnsupportedColorProfile)?;
    let profile = color.profile(icc.as_deref())?;
    let orientation = decoder.orientation().map_err(|_| Error::InvalidImage)?;
    if facts.animated {
        drop(decoder);
        crate::source_apng::decode(
            bytes,
            limits,
            &facts,
            &delays,
            (orientation, profile.as_ref()),
            emit,
        )?;
    } else {
        let pixels = DynamicImage::from_decoder(decoder).map_err(|_| Error::InvalidImage)?;
        let pixels = normalize_dynamic_with_profile(pixels, orientation, profile.as_ref())?;
        emit(pixels, 0, Orientation::NoTransforms, None)?;
    }
    Ok((
        if facts.animated {
            SourceFormat::Apng
        } else {
            SourceFormat::Png
        },
        facts,
        orientation.to_exif(),
    ))
}
