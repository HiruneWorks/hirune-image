use crate::{
    source::{Description, Emit, static_facts},
    timeline::Timeline,
};
use application::{
    error::{ProcessingError as Error, Result},
    images::{Limits, SourceFormat},
};
use image::{RgbaImage, metadata::Orientation};
use jxl_oxide::{AllocTracker, EnumColourEncoding, InitializeResult, JxlImage, RenderingIntent};

pub(crate) fn decode(bytes: &[u8], limits: &Limits, emit: &mut Emit<'_>) -> Result<Description> {
    let tracker = AllocTracker::with_limit(
        usize::try_from(limits.working_bytes).map_err(|_| Error::ImageLimitsExceeded)?,
    );
    let mut pending = JxlImage::builder().alloc_tracker(tracker).build_uninit();
    let mut offset = 0;
    let mut image = loop {
        if offset == bytes.len() {
            return Err(Error::InvalidImage);
        }
        let end = (offset + 256).min(bytes.len());
        let consumed = pending
            .feed_bytes(&bytes[offset..end])
            .map_err(|_| Error::InvalidImage)?;
        offset += consumed;
        match pending.try_init().map_err(|_| Error::InvalidImage)? {
            InitializeResult::Initialized(image) => break image,
            InitializeResult::NeedMoreData(uninit) => {
                if consumed == 0 {
                    return Err(Error::InvalidImage);
                }
                pending = uninit;
            }
        }
    };
    let mut facts = static_facts(image.width(), image.height());
    limits.preflight(&facts)?;
    if image.hdr_type().is_some() {
        return Err(Error::UnsupportedColorProfile);
    }
    if image
        .image_header()
        .metadata
        .ec_info
        .iter()
        .any(|channel| channel.alpha_associated() == Some(true))
    {
        return Err(Error::UnsupportedColorProfile);
    }
    if let Some(icc) = image.original_icc() {
        crate::normalize::profile(icc)?;
    }
    let animation = image
        .image_header()
        .metadata
        .animation
        .as_ref()
        .map(|a| (a.num_loops, a.tps_numerator, a.tps_denominator));
    if let Some((loops, _, _)) = animation.as_ref() {
        facts.animated = true;
        facts.loop_count = u16::try_from(*loops).map_err(|_| Error::UnrepresentableAnimation)?;
    }
    image.request_color_encoding(EnumColourEncoding::srgb(RenderingIntent::Relative));
    while offset < bytes.len() {
        let end = (offset + 4096).min(bytes.len());
        let consumed = image
            .feed_bytes(&bytes[offset..end])
            .map_err(|_| Error::InvalidImage)?;
        if consumed == 0 {
            return Err(Error::InvalidImage);
        }
        offset += consumed;
        if image.num_loaded_keyframes() > limits.frames as usize {
            return Err(Error::ImageLimitsExceeded);
        }
    }
    image.finalize().map_err(|_| Error::InvalidImage)?;
    if !image.is_loading_done() {
        return Err(Error::InvalidImage);
    }
    facts.frame_count =
        u32::try_from(image.num_loaded_keyframes()).map_err(|_| Error::ImageLimitsExceeded)?;
    if !facts.animated && facts.frame_count != 1 {
        return Err(Error::UnrepresentableAnimation);
    }
    let mut clock = Timeline::default();
    let mut delays = Vec::new();
    for index in 0..facts.frame_count as usize {
        let header = image.frame_header(index).ok_or(Error::InvalidImage)?;
        let delay = if let Some((_, numerator, denominator)) = animation.as_ref() {
            let numerator_ticks = u64::from(header.duration)
                .checked_mul(u64::from(*denominator))
                .ok_or(Error::UnrepresentableAnimation)?;
            clock.push(numerator_ticks, u64::from(*numerator))?
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
    for (index, delay) in delays.into_iter().enumerate() {
        let render = image.render_frame(index).map_err(|_| Error::InvalidImage)?;
        let mut stream = render.stream();
        let channels = stream.channels() as usize;
        if !(1..=4).contains(&channels) {
            return Err(Error::UnsupportedColorProfile);
        }
        let pixels = u64::from(stream.width()) * u64::from(stream.height());
        limits.canvas(stream.width(), stream.height())?;
        let mut samples = vec![0u8; pixels as usize * channels];
        if stream.write_to_buffer(&mut samples) != samples.len() {
            return Err(Error::InvalidImage);
        }
        let mut rgba = Vec::with_capacity(pixels as usize * 4);
        for pixel in samples.chunks_exact(channels) {
            match channels {
                1 => rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], 255]),
                2 => rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]),
                3 => rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]),
                4 => rgba.extend_from_slice(pixel),
                _ => unreachable!(),
            }
        }
        let pixels = RgbaImage::from_raw(stream.width(), stream.height(), rgba)
            .ok_or(Error::InvalidImage)?;
        drop(stream);
        drop(render);
        emit(pixels, delay, Orientation::NoTransforms, None)?;
        // Release display-frame caches. The decoder retains bounded parsing and reference state.
        let region = image.current_image_region();
        image.set_image_region(region);
    }
    Ok((SourceFormat::Jxl, facts, 1))
}
