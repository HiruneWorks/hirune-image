use crate::container::webp_structure;
use application::{
    error::{ProcessingError as Error, Result},
    images::{
        Encoding, ImageFacts, Limits, MAX_WEBP_DIMENSION, MAX_WEBP_FILE_BYTES, ProcessorInfo,
    },
};
use libwebp_sys as ffi;
use std::{
    mem,
    ptr::{self, NonNull},
};

pub fn processor_info() -> ProcessorInfo {
    fn version(v: i32) -> String {
        format!("{}.{}.{}", v >> 16, (v >> 8) & 255, v & 255)
    }
    // Version queries use no input pointers and allocate no codec storage.
    unsafe {
        ProcessorInfo {
            version: env!("CARGO_PKG_VERSION").into(),
            build_id: format!(
                "{}:{}",
                option_env!("HIRUNE_BUILD_ID").unwrap_or("development"),
                env!("HIRUNE_SOURCE_FINGERPRINT")
            ),
            pipeline_version: "4-png-sdr-color-ancillary-recovery".into(),
            image: "0.25.10".into(),
            gif: "0.14.2".into(),
            heic: "heic-rs/0.1.4".into(),
            jxl: "jxl-oxide/0.12.6".into(),
            svg: "resvg/0.49.0".into(),
            libavif: jc_libavif_sys::UPSTREAM_LIBAVIF_VERSION.into(),
            libaom: jc_libavif_sys::UPSTREAM_LIBAOM_VERSION.into(),
            libwebp_encoder: version(ffi::WebPGetEncoderVersion()),
            libwebp_decoder: version(ffi::WebPGetDecoderVersion()),
            libwebp_demux: version(ffi::WebPGetDemuxVersion()),
            libwebp_mux: version(ffi::WebPGetMuxVersion()),
            color_engine: "moxcms/0.9.1".into(),
        }
    }
}

pub fn probe() -> Result<ProcessorInfo> {
    for encoding in [
        Encoding::Lossless { effort: 0 },
        Encoding::Lossy {
            quality: 75,
            effort: 0,
        },
    ] {
        let bytes = encode(&[255; 64], 4, 4, &encoding, 65536)?;
        let facts = decode_frames(&bytes, &Limits::default(), |pixels, _| {
            if pixels.len() != 64 {
                return Err(Error::ProcessorUnavailable);
            }
            Ok(())
        })?;
        if facts.width != 4 || facts.height != 4 {
            return Err(Error::ProcessorUnavailable);
        }
    }
    Ok(processor_info())
}

struct Picture(ffi::WebPPicture);
impl Drop for Picture {
    fn drop(&mut self) {
        unsafe { ffi::WebPPictureFree(&mut self.0) }
    }
}
struct Writer {
    bytes: Vec<u8>,
    maximum: usize,
    failed: bool,
}
extern "C" fn write_chunk(data: *const u8, length: usize, picture: *const ffi::WebPPicture) -> i32 {
    // libwebp calls this synchronously. The picture and its Writer remain alive until it returns.
    let writer = unsafe { &mut *((*picture).custom_ptr as *mut Writer) };
    let Some(total) = writer.bytes.len().checked_add(length) else {
        writer.failed = true;
        return 0;
    };
    if total > writer.maximum || writer.bytes.try_reserve(length).is_err() {
        writer.failed = true;
        return 0;
    }
    if length > 0 {
        writer
            .bytes
            .extend_from_slice(unsafe { std::slice::from_raw_parts(data, length) });
    }
    1
}

pub fn encoding_config(encoding: &Encoding) -> Result<ffi::WebPConfig> {
    let mut config = unsafe { mem::zeroed() };
    if unsafe { ffi::WebPConfigInit(&mut config) } != 1 {
        return Err(Error::ProcessorUnavailable);
    }
    config.lossless = i32::from(matches!(encoding, Encoding::Lossless { .. }));
    config.quality = match encoding {
        Encoding::Lossy { quality, .. } => f32::from(*quality),
        Encoding::Lossless { .. } => 75.,
    };
    config.method = i32::from(encoding.effort());
    config.thread_level = 0;
    config.alpha_quality = 100;
    config.exact = 1;
    if unsafe { ffi::WebPValidateConfig(&config) } != 1 {
        return Err(Error::InvalidRecipe);
    }
    Ok(config)
}

pub fn encode(
    rgba: &[u8],
    width: u32,
    height: u32,
    encoding: &Encoding,
    maximum: u64,
) -> Result<Vec<u8>> {
    if width == 0
        || height == 0
        || width > MAX_WEBP_DIMENSION
        || height > MAX_WEBP_DIMENSION
        || rgba.len() as u64 != u64::from(width) * u64::from(height) * 4
    {
        return Err(Error::InternalError);
    }
    let config = encoding_config(encoding)?;
    let mut picture = Picture(unsafe { mem::zeroed() });
    if unsafe { ffi::WebPPictureInit(&mut picture.0) } != 1 {
        return Err(Error::ProcessorUnavailable);
    }
    picture.0.width = width as i32;
    picture.0.height = height as i32;
    picture.0.use_argb = 1;
    let mut writer = Writer {
        bytes: Vec::new(),
        maximum: usize::try_from(maximum.min(MAX_WEBP_FILE_BYTES).min(isize::MAX as u64))
            .map_err(|_| Error::OutputLimitsExceeded)?,
        failed: false,
    };
    picture.0.writer = Some(write_chunk);
    picture.0.custom_ptr = (&mut writer as *mut Writer).cast();
    // Import copies the bounded RGBA data into picture storage. Picture frees that storage on drop.
    if unsafe { ffi::WebPPictureImportRGBA(&mut picture.0, rgba.as_ptr(), (width * 4) as i32) } != 1
    {
        return Err(Error::ImageLimitsExceeded);
    }
    if unsafe { ffi::WebPEncode(&config, &mut picture.0) } != 1 {
        return Err(if writer.failed {
            Error::OutputLimitsExceeded
        } else {
            Error::ProcessorUnavailable
        });
    }
    Ok(writer.bytes)
}

pub use crate::animation::AnimationMux;

struct Decoder(NonNull<ffi::WebPAnimDecoder>);
impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { ffi::WebPAnimDecoderDelete(self.0.as_ptr()) }
    }
}

pub fn decode_frames(
    bytes: &[u8],
    limits: &Limits,
    mut consume: impl FnMut(Vec<u8>, u32) -> Result<()>,
) -> Result<ImageFacts> {
    limits.validate()?;
    let structure = webp_structure(bytes, limits)?;
    let facts = structure.facts;
    limits.preflight(&facts)?;
    let mut options = unsafe { mem::zeroed() };
    if unsafe { ffi::WebPAnimDecoderOptionsInit(&mut options) } != 1 {
        return Err(Error::ProcessorUnavailable);
    }
    options.color_mode = ffi::MODE_RGBA;
    options.use_threads = 0;
    let data = ffi::WebPData {
        bytes: bytes.as_ptr(),
        size: bytes.len(),
    };
    // The checks above enforce dimensions and frame budgets before the native decoder allocates a canvas.
    let decoder = Decoder(
        NonNull::new(unsafe { ffi::WebPAnimDecoderNew(&data, &options) })
            .ok_or(Error::InvalidImage)?,
    );
    let mut info = unsafe { mem::zeroed::<ffi::WebPAnimInfo>() };
    if unsafe { ffi::WebPAnimDecoderGetInfo(decoder.0.as_ptr(), &mut info) } != 1
        || info.canvas_width != facts.width
        || info.canvas_height != facts.height
        || info.frame_count != facts.frame_count
    {
        return Err(Error::InvalidImage);
    }
    let frame_bytes = u64::from(facts.width)
        .checked_mul(u64::from(facts.height))
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .filter(|&bytes| bytes <= isize::MAX as usize)
        .ok_or(Error::ImageLimitsExceeded)?;
    let mut previous = 0;
    for delay in structure.delays {
        if unsafe { ffi::WebPAnimDecoderHasMoreFrames(decoder.0.as_ptr()) } != 1 {
            return Err(Error::InvalidImage);
        }
        let mut output = ptr::null_mut();
        let mut timestamp = 0;
        if unsafe { ffi::WebPAnimDecoderGetNext(decoder.0.as_ptr(), &mut output, &mut timestamp) }
            != 1
            || output.is_null()
        {
            return Err(Error::InvalidImage);
        }
        if facts.animated && (timestamp < previous || (timestamp - previous) as u32 != delay) {
            return Err(Error::UnrepresentableAnimation);
        }
        previous = timestamp;
        consume(
            unsafe { std::slice::from_raw_parts(output, frame_bytes) }.to_vec(),
            delay,
        )?;
    }
    if unsafe { ffi::WebPAnimDecoderHasMoreFrames(decoder.0.as_ptr()) } != 0 {
        return Err(Error::InvalidImage);
    }
    Ok(facts)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effort_is_applied_and_invalid_native_values_rejected() {
        for effort in 0..=6 {
            assert_eq!(
                encoding_config(&Encoding::Lossless { effort })
                    .unwrap()
                    .method,
                i32::from(effort)
            );
        }
        assert_eq!(
            encoding_config(&Encoding::Lossless { effort: 99 }).unwrap_err(),
            Error::InvalidRecipe
        );
    }
}
