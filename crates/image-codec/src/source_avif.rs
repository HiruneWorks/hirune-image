use crate::{
    normalize::{cicp_icc, normalize_dynamic},
    source::{Description, Emit, static_facts},
    source_heic,
    timeline::Timeline,
};
use application::{
    error::{ProcessingError as Error, Result},
    images::{Limits, SourceFormat},
};
use image::{DynamicImage, metadata::Orientation};
use jc_libavif_sys as ffi;
use std::ptr::NonNull;

struct Decoder(NonNull<ffi::avifDecoder>);
impl Drop for Decoder {
    fn drop(&mut self) {
        // This pointer owns the decoder, its native image and its borrowed-input IO wrapper.
        unsafe { ffi::avifDecoderDestroy(self.0.as_ptr()) };
    }
}
fn check(result: ffi::avifResult) -> Result<()> {
    if result == ffi::avifResult_AVIF_RESULT_OK {
        Ok(())
    } else {
        Err(Error::InvalidImage)
    }
}
unsafe fn metadata(data: &ffi::avifRWData) -> Result<&[u8]> {
    if data.size > 1 << 20 {
        return Err(Error::UnsupportedColorProfile);
    }
    if data.size == 0 {
        return Ok(&[]);
    }
    if data.data.is_null() {
        return Err(Error::InvalidImage);
    }
    // libavif owns this buffer until the next decoder call. The caller uses it synchronously.
    Ok(unsafe { std::slice::from_raw_parts(data.data, data.size) })
}

pub(crate) fn decode(bytes: &[u8], limits: &Limits, emit: &mut Emit<'_>) -> Result<Description> {
    let container = source_heic::inspect(bytes)?;
    let decoder = Decoder(
        NonNull::new(unsafe { ffi::avifDecoderCreate() }).ok_or(Error::ProcessorUnavailable)?,
    );
    let raw = decoder.0.as_ptr();
    // Source bytes outlive the decoder. Header budgets are set before parsing and native allocation.
    unsafe {
        (*raw).maxThreads = 1;
        (*raw).imageSizeLimit = limits
            .canvas_pixels
            .min(limits.working_bytes / 20)
            .min(u64::from(u32::MAX)) as u32;
        (*raw).imageDimensionLimit = limits.input_dimension;
        (*raw).imageCountLimit = limits.frames;
        (*raw).ignoreXMP = 1;
        check(ffi::avifDecoderSetSource(
            raw,
            if container.tracks {
                ffi::avifDecoderSource_AVIF_DECODER_SOURCE_TRACKS
            } else {
                ffi::avifDecoderSource_AVIF_DECODER_SOURCE_PRIMARY_ITEM
            },
        ))?;
        check(ffi::avifDecoderSetIOMemory(
            raw,
            bytes.as_ptr(),
            bytes.len(),
        ))?;
        check(ffi::avifDecoderParse(raw))?;
    }
    let native = unsafe { raw.read() };
    if native.image.is_null() || native.imageCount <= 0 {
        return Err(Error::InvalidImage);
    }
    let header = unsafe { &*native.image };
    let mut facts = static_facts(header.width, header.height);
    facts.animated = container.tracks;
    facts.frame_count = u32::try_from(native.imageCount).map_err(|_| Error::InvalidImage)?;
    if !facts.animated && facts.frame_count != 1 {
        return Err(Error::UnrepresentableAnimation);
    }
    if facts.animated {
        facts.loop_count = match native.repetitionCount {
            ffi::AVIF_REPETITION_COUNT_INFINITE => 0,
            ffi::AVIF_REPETITION_COUNT_UNKNOWN => 1,
            count if count >= 0 => {
                u16::try_from(i64::from(count) + 1).map_err(|_| Error::UnrepresentableAnimation)?
            }
            _ => return Err(Error::UnrepresentableAnimation),
        };
    }
    let mut clock = Timeline::default();
    let mut delays = Vec::new();
    for index in 0..facts.frame_count {
        let delay = if facts.animated {
            let mut timing = ffi::avifImageTiming::default();
            check(unsafe { ffi::avifDecoderNthImageTiming(raw, index, &mut timing) })?;
            clock.push(timing.durationInTimescales, timing.timescale)?
        } else {
            0
        };
        facts.duration_ms = facts
            .duration_ms
            .checked_add(delay)
            .ok_or(Error::ImageLimitsExceeded)?;
        delays.push(delay);
    }
    limits.preflight(&facts)?;
    let mut display_size = None;
    for delay in delays {
        check(unsafe { ffi::avifDecoderNextImage(raw) })?;
        let native_image = unsafe { &*(*raw).image };
        limits.canvas(native_image.width, native_image.height)?;
        if matches!(native_image.transferCharacteristics, 16 | 18) {
            return Err(Error::UnsupportedColorProfile);
        }
        let mut rgb = ffi::avifRGBImage::default();
        unsafe { ffi::avifRGBImageSetDefaults(&mut rgb, native_image) };
        rgb.format = ffi::avifRGBFormat_AVIF_RGB_FORMAT_RGBA;
        rgb.depth = if native_image.depth > 8 { 16 } else { 8 };
        rgb.alphaPremultiplied = 0;
        rgb.maxThreads = 1;
        let row_bytes = u64::from(rgb.width)
            .checked_mul(4 * u64::from(rgb.depth / 8))
            .ok_or(Error::ImageLimitsExceeded)?;
        let size = row_bytes
            .checked_mul(u64::from(rgb.height))
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| *n <= isize::MAX as usize)
            .ok_or(Error::ImageLimitsExceeded)?;
        let mut samples = vec![0u8; size];
        rgb.rowBytes = u32::try_from(row_bytes).map_err(|_| Error::ImageLimitsExceeded)?;
        rgb.pixels = samples.as_mut_ptr();
        // The native conversion writes only into this checked Rust-owned buffer and retains no pointer.
        check(unsafe { ffi::avifImageYUVToRGB(native_image, &mut rgb) })?;
        let dynamic = if rgb.depth == 16 {
            let data = samples
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_ne_bytes([b[0], b[1]]))
                .collect();
            DynamicImage::ImageRgba16(
                image::ImageBuffer::from_raw(rgb.width, rgb.height, data)
                    .ok_or(Error::InvalidImage)?,
            )
        } else {
            DynamicImage::ImageRgba8(
                image::RgbaImage::from_raw(rgb.width, rgb.height, samples)
                    .ok_or(Error::InvalidImage)?,
            )
        };
        let icc = unsafe { metadata(&native_image.icc)? };
        let generated;
        let icc = if icc.is_empty() {
            generated = cicp_icc(
                native_image.colorPrimaries,
                native_image.transferCharacteristics,
            )?;
            generated.as_slice()
        } else {
            icc
        };
        let mut pixels = normalize_dynamic(dynamic, Orientation::NoTransforms, Some(icc))?;
        if native_image.transformFlags & ffi::avifTransformFlag_AVIF_TRANSFORM_CLAP != 0 {
            let mut rect = ffi::avifCropRect::default();
            if unsafe {
                ffi::avifCropRectFromCleanApertureBox(
                    &mut rect,
                    &native_image.clap,
                    pixels.width(),
                    pixels.height(),
                    std::ptr::null_mut(),
                )
            } == 0
                || rect
                    .x
                    .checked_add(rect.width)
                    .is_none_or(|x| x > pixels.width())
                || rect
                    .y
                    .checked_add(rect.height)
                    .is_none_or(|y| y > pixels.height())
            {
                return Err(Error::InvalidImage);
            }
            pixels = image::imageops::crop_imm(&pixels, rect.x, rect.y, rect.width, rect.height)
                .to_image();
        }
        if native_image.transformFlags & ffi::avifTransformFlag_AVIF_TRANSFORM_IROT != 0 {
            pixels = match native_image.irot.angle {
                0 => pixels,
                1 => image::imageops::rotate270(&pixels),
                2 => image::imageops::rotate180(&pixels),
                3 => image::imageops::rotate90(&pixels),
                _ => return Err(Error::InvalidImage),
            };
        }
        if native_image.transformFlags & ffi::avifTransformFlag_AVIF_TRANSFORM_IMIR != 0 {
            pixels = match native_image.imir.axis {
                0 => image::imageops::flip_vertical(&pixels),
                1 => image::imageops::flip_horizontal(&pixels),
                _ => return Err(Error::InvalidImage),
            };
        }
        if display_size.is_some_and(|size| size != pixels.dimensions()) {
            return Err(Error::UnrepresentableAnimation);
        }
        display_size = Some(pixels.dimensions());
        emit(pixels, delay, Orientation::NoTransforms, None)?;
    }
    let (width, height) = display_size.ok_or(Error::InvalidImage)?;
    facts.width = width;
    facts.height = height;
    Ok((SourceFormat::Avif, facts, 1))
}
