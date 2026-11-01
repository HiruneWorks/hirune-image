use crate::{normalize::normalize_dynamic, source::Emit};
use application::{
    error::{ProcessingError as Error, Result},
    images::{ImageFacts, Limits},
};
use image::{DynamicImage, ImageBuffer, Rgba, metadata::Orientation};
use std::io::Cursor;

pub(crate) fn decode(
    bytes: &[u8],
    limits: &Limits,
    facts: &ImageFacts,
    delays: &[u32],
    normalization: (Orientation, Option<&[u8]>),
    emit: &mut Emit<'_>,
) -> Result<()> {
    let pixels = limits.canvas(facts.width, facts.height)?;
    if pixels.checked_mul(48).ok_or(Error::ImageLimitsExceeded)? > limits.working_bytes {
        return Err(Error::ImageLimitsExceeded);
    }
    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(bytes),
        png::Limits {
            bytes: limits.working_bytes.min(2 << 20) as usize,
        },
    );
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|_| Error::InvalidImage)?;
    let hidden = reader.info().frame_control.is_none();
    if hidden {
        let full_work = ImageFacts {
            frame_count: facts
                .frame_count
                .checked_add(1)
                .ok_or(Error::ImageLimitsExceeded)?,
            ..facts.clone()
        };
        limits.preflight(&full_work)?;
    }
    let mut raw = vec![0; pixels as usize * 8];
    let mut canvas = ImageBuffer::<Rgba<u16>, Vec<u16>>::new(facts.width, facts.height);
    let count = facts
        .frame_count
        .checked_add(u32::from(hidden))
        .ok_or(Error::ImageLimitsExceeded)?;
    let mut displayed = 0usize;
    for index in 0..count {
        let output = reader
            .next_frame(&mut raw)
            .map_err(|_| Error::InvalidImage)?;
        if hidden && index == 0 {
            continue;
        }
        let control = reader.info().frame_control.ok_or(Error::InvalidImage)?;
        if control.width != output.width
            || control.height != output.height
            || control
                .x_offset
                .checked_add(control.width)
                .is_none_or(|x| x > facts.width)
            || control
                .y_offset
                .checked_add(control.height)
                .is_none_or(|y| y > facts.height)
        {
            return Err(Error::InvalidImage);
        }
        let previous = if control.dispose_op == png::DisposeOp::Previous {
            Some(canvas.clone())
        } else {
            None
        };
        let channels = output.color_type.samples();
        let bytes_per_sample = if output.bit_depth == png::BitDepth::Sixteen {
            2
        } else {
            1
        };
        let stride = channels * bytes_per_sample;
        let data = &raw[..output.buffer_size()];
        for (index, encoded) in data.chunks_exact(stride).enumerate() {
            let mut samples = [0u16; 4];
            for (channel, sample) in encoded.chunks_exact(bytes_per_sample).enumerate() {
                samples[channel] = if bytes_per_sample == 2 {
                    u16::from_be_bytes([sample[0], sample[1]])
                } else {
                    u16::from(sample[0]) * 257
                };
            }
            let source = match output.color_type {
                png::ColorType::Rgba => samples,
                png::ColorType::Rgb => [samples[0], samples[1], samples[2], 65535],
                png::ColorType::Grayscale => [samples[0], samples[0], samples[0], 65535],
                png::ColorType::GrayscaleAlpha => [samples[0], samples[0], samples[0], samples[1]],
                png::ColorType::Indexed => return Err(Error::InvalidImage),
            };
            let x = index as u32 % output.width + control.x_offset;
            let y = index as u32 / output.width + control.y_offset;
            let target = canvas.get_pixel_mut(x, y);
            target.0 = if control.blend_op == png::BlendOp::Source {
                source
            } else {
                over(source, target.0)
            };
        }
        let pixels = normalize_dynamic(
            DynamicImage::ImageRgba16(canvas.clone()),
            normalization.0,
            normalization.1,
        )?;
        emit(
            pixels,
            *delays.get(displayed).ok_or(Error::InvalidImage)?,
            Orientation::NoTransforms,
            None,
        )?;
        displayed += 1;
        match control.dispose_op {
            png::DisposeOp::None => {}
            png::DisposeOp::Previous => canvas = previous.ok_or(Error::InternalError)?,
            png::DisposeOp::Background => {
                for y in control.y_offset..control.y_offset + control.height {
                    for x in control.x_offset..control.x_offset + control.width {
                        canvas.put_pixel(x, y, Rgba([0; 4]));
                    }
                }
            }
        }
    }
    reader.finish().map_err(|_| Error::InvalidImage)?;
    if displayed != delays.len() {
        return Err(Error::InvalidImage);
    }
    Ok(())
}

fn over(source: [u16; 4], destination: [u16; 4]) -> [u16; 4] {
    let max = u64::from(u16::MAX);
    let sa = u64::from(source[3]);
    let da = u64::from(destination[3]);
    let alpha = sa * max + da * (max - sa);
    if alpha == 0 {
        return [0; 4];
    }
    let mut output = [0; 4];
    for channel in 0..3 {
        let value = u64::from(source[channel]) * sa * max
            + u64::from(destination[channel]) * da * (max - sa);
        output[channel] = ((value + alpha / 2) / alpha) as u16;
    }
    output[3] = ((alpha + max / 2) / max) as u16;
    output
}
