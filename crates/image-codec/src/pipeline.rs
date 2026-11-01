use crate::{
    container::webp_structure,
    resize::resize_inside,
    source::decode_source,
    webp::{self, AnimationMux},
};
use application::{
    error::{ProcessingError as Error, Result},
    images::{
        ImageFacts, JobInput, Limits, Manifest, OutputFacts, inside_dimensions, sha256,
        validate_recipe,
    },
};
use std::{fs, path::Path};

enum EncodedVariant {
    Static(Vec<u8>),
    Animated(AnimationMux),
}

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
    let bytes = fs::read(&source_path).map_err(|_| Error::InternalError)?;
    if sha256(&bytes) != job.source_sha256 {
        return Err(Error::InternalError);
    }
    let mut variants = Vec::new();
    let mut dimensions = Vec::new();
    let mut aggregate = 0u64;
    let mut frame_count = 0u32;
    let mut decoder_starts = 0u32;
    let mut animated = false;
    let decoded = decode_source(
        &bytes,
        &job.limits,
        || decoder_starts += 1,
        |pixels, delay| {
            if frame_count == 0 {
                animated = delay > 0;
                for output in &job.recipe.outputs {
                    let size = inside_dimensions(
                        pixels.width(),
                        pixels.height(),
                        output.max_width,
                        output.max_height,
                    );
                    dimensions.push(size);
                    variants.push(if animated {
                        let mux = AnimationMux::new(size.0, size.1, job.limits.output_bytes)?;
                        aggregate = aggregate
                            .checked_add(mux.len())
                            .ok_or(Error::OutputLimitsExceeded)?;
                        if aggregate > job.limits.output_bytes {
                            return Err(Error::OutputLimitsExceeded);
                        }
                        EncodedVariant::Animated(mux)
                    } else {
                        EncodedVariant::Static(Vec::new())
                    });
                }
            } else if !animated || delay == 0 {
                return Err(Error::UnrepresentableAnimation);
            }
            for ((variant, &(width, height)), output) in variants
                .iter_mut()
                .zip(&dimensions)
                .zip(&job.recipe.outputs)
            {
                let remaining = job
                    .limits
                    .output_bytes
                    .checked_sub(aggregate)
                    .ok_or(Error::OutputLimitsExceeded)?;
                let resized = resize_inside(&pixels, width, height);
                let encoded =
                    webp::encode(resized.as_raw(), width, height, &output.encoding, remaining)?;
                drop(resized);
                let added = match variant {
                    EncodedVariant::Static(data) => {
                        let added = encoded.len() as u64;
                        *data = encoded;
                        added
                    }
                    EncodedVariant::Animated(mux) => {
                        let before = mux.len();
                        mux.add(&encoded, delay, remaining)?;
                        mux.len() - before
                    }
                };
                aggregate = aggregate
                    .checked_add(added)
                    .ok_or(Error::OutputLimitsExceeded)?;
                if aggregate > job.limits.output_bytes {
                    return Err(Error::OutputLimitsExceeded);
                }
            }
            frame_count = frame_count
                .checked_add(1)
                .ok_or(Error::ImageLimitsExceeded)?;
            Ok(())
        },
    )?;
    drop(bytes);
    if frame_count == 0
        || frame_count != decoded.facts.image.frame_count
        || animated != decoded.facts.image.animated
    {
        return Err(Error::InvalidImage);
    }
    let mut outputs = Vec::new();
    for ((variant, (width, height)), output) in variants
        .into_iter()
        .zip(dimensions)
        .zip(&job.recipe.outputs)
    {
        let encoded = match variant {
            EncodedVariant::Static(data) => data,
            EncodedVariant::Animated(mux) => mux.finish(decoded.facts.image.loop_count)?,
        };
        let expected = ImageFacts {
            width,
            height,
            ..decoded.facts.image.clone()
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
    fs::remove_file(source_path).map_err(|_| Error::InternalError)?;
    fs::write(directory.join("metrics.json"), format!("{{\"source_decoder_starts\":{decoder_starts},\"decoded_frames\":{frame_count},\"raw_frame_spool_bytes\":0,\"output_count\":{}}}", outputs.len())).map_err(|_| Error::InternalError)?;
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
