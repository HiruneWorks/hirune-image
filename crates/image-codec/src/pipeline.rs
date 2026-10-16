use crate::{
    container::webp_structure,
    resize::resize_inside,
    source::decode_source,
    webp::{self, AnimationMux},
};
use application::{
    error::{ProcessingError as Error, Result},
    images::{
        AnimationPolicy, ImageFacts, JobInput, Limits, Manifest, OutputFacts, inside_dimensions,
        sha256, validate_recipe,
    },
};
use image::RgbaImage;
use std::{fs, path::Path};

pub fn process_job(directory: &Path, job: &JobInput) -> Result<Manifest> {
    job.limits.validate()?;
    validate_recipe(&job.recipe, &job.limits)?;
    let source_path = directory.join("source");
    if fs::metadata(&source_path)
        .map_err(|_| Error::InternalError)?
        .len()
        > job.limits.source_bytes
    {
        return Err(Error::RequestTooLarge);
    }
    let bytes = fs::read(source_path).map_err(|_| Error::InternalError)?;
    if sha256(&bytes) != job.source_sha256 {
        return Err(Error::InternalError);
    }
    let mut index = 0u32;
    let mut decoder_starts = 0u32;
    let decoded = decode_source(
        &bytes,
        &job.limits,
        || decoder_starts += 1,
        |pixels, _delay| {
            fs::write(
                directory.join(format!("frame-{index}.rgba")),
                pixels.as_raw(),
            )
            .map_err(|_| Error::InternalError)?;
            index += 1;
            Ok(())
        },
    )?;
    drop(bytes);
    let animated = decoded.facts.image.frame_count > 1;
    if animated && job.recipe.animation_policy == AnimationPolicy::Reject {
        return Err(Error::AnimationNotAllowed);
    }
    let preserve = animated && job.recipe.animation_policy == AnimationPolicy::Preserve;
    let count = if preserve {
        decoded.facts.image.frame_count
    } else {
        1
    };
    let (source_width, source_height) = decoded.normalized_dimensions;
    let mut outputs = Vec::new();
    let mut aggregate = 0u64;
    for output in &job.recipe.outputs {
        let (width, height) = inside_dimensions(
            source_width,
            source_height,
            output.max_width,
            output.max_height,
        );
        let remaining = job
            .limits
            .output_bytes
            .checked_sub(aggregate)
            .ok_or(Error::OutputLimitsExceeded)?;
        let mut mux = if preserve {
            Some(AnimationMux::new(
                decoded.facts.image.loop_count,
                remaining,
            )?)
        } else {
            None
        };
        let mut encoded = Vec::new();
        for index in 0..count {
            let raw = fs::read(directory.join(format!("frame-{index}.rgba")))
                .map_err(|_| Error::InternalError)?;
            let pixels = RgbaImage::from_raw(source_width, source_height, raw)
                .ok_or(Error::InternalError)?;
            let resized = resize_inside(&pixels, width, height);
            drop(pixels);
            let frame = webp::encode(resized.as_raw(), width, height, &output.encoding, remaining)?;
            if let Some(mux) = mux.as_mut() {
                mux.add(&frame, decoded.delays[index as usize])?
            } else {
                encoded = frame
            }
        }
        if let Some(mux) = mux {
            encoded = mux.finish()?
        }
        aggregate = aggregate
            .checked_add(encoded.len() as u64)
            .ok_or(Error::OutputLimitsExceeded)?;
        if aggregate > job.limits.output_bytes {
            return Err(Error::OutputLimitsExceeded);
        }
        let expected = ImageFacts {
            width,
            height,
            frame_count: count,
            duration_ms: if preserve {
                decoded.facts.image.duration_ms
            } else {
                0
            },
            loop_count: if preserve {
                decoded.facts.image.loop_count
            } else {
                1
            },
        };
        let actual = verify_output(&encoded, &job.limits)?;
        if actual != expected {
            return Err(Error::UnrepresentableAnimation);
        }
        fs::write(
            directory.join(format!("output-{}.webp", output.id)),
            &encoded,
        )
        .map_err(|_| Error::InternalError)?;
        outputs.push(OutputFacts {
            id: output.id.clone(),
            media_type: application::images::MediaType::Webp,
            byte_size: encoded.len() as u64,
            sha256: sha256(&encoded),
            image: actual,
        });
    }
    for frame_index in 0..index {
        fs::remove_file(directory.join(format!("frame-{frame_index}.rgba")))
            .map_err(|_| Error::InternalError)?;
    }
    fs::remove_file(directory.join("source")).map_err(|_| Error::InternalError)?;
    fs::write(
        directory.join("metrics.json"),
        format!(
            "{{\"source_decoder_starts\":{decoder_starts},\"decoded_frames\":{index},\"output_count\":{}}}",
            outputs.len()
        ),
    )
    .map_err(|_| Error::InternalError)?;
    Ok(Manifest {
        contract_version: 1,
        processor: webp::processor_info(),
        source_sha256: job.source_sha256.clone(),
        recipe_sha256: job.recipe_sha256.clone(),
        source: decoded.facts,
        outputs,
    })
}
pub fn verify_output(bytes: &[u8], limits: &Limits) -> Result<ImageFacts> {
    let structure = webp_structure(bytes, limits)?;
    if structure.metadata
        || structure.facts.width > limits.output_dimension
        || structure.facts.height > limits.output_dimension
    {
        return Err(Error::InvalidImage);
    }
    webp::decode_frames(bytes, limits, |_, _| Ok(()))
}
