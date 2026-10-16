use application::{
    error::ProcessingError,
    images::{JobInput, JobResult, Limits},
};
use std::{fs, path::PathBuf};
fn main() {
    if run().is_err() {
        std::process::exit(1)
    }
}
fn run() -> Result<(), ProcessingError> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let (mode, directory, limits_json) = match arguments.as_slice() {
        [mode, flag, limits] if mode == "--probe" && flag == "--limits" => (mode, None, limits),
        [mode, directory, flag, limits] if mode == "--job" && flag == "--limits" => {
            (mode, Some(PathBuf::from(directory)), limits)
        }
        _ => return Err(ProcessingError::InvalidRequest),
    };
    let limits_json = limits_json
        .to_str()
        .ok_or(ProcessingError::InvalidRequest)?;
    if limits_json.len() > 8192 {
        return Err(ProcessingError::InvalidRequest);
    }
    let limits: Limits =
        serde_json::from_str(limits_json).map_err(|_| ProcessingError::InvalidRequest)?;
    limits.validate()?;
    // Trusted supervisor settings fence allocations before reading request-derived job metadata.
    image_codec::sandbox::apply_resource_limits(
        limits.child_memory_bytes,
        limits.child_cpu_seconds,
    )?;
    if mode == "--probe" {
        // Probe both modes and the actual native libraries before accepting traffic.
        print!(
            "{}",
            serde_json::to_string(&image_codec::probe()?)
                .map_err(|_| ProcessingError::InternalError)?
        );
        return Ok(());
    }
    let directory = directory.ok_or(ProcessingError::InvalidRequest)?;
    let path = directory.join("job.json");
    if fs::metadata(&path)
        .map_err(|_| ProcessingError::InvalidRequest)?
        .len()
        > limits.job_metadata_bytes()?
    {
        return Err(ProcessingError::InvalidRequest);
    }
    let job: JobInput =
        serde_json::from_slice(&fs::read(path).map_err(|_| ProcessingError::InvalidRequest)?)
            .map_err(|_| ProcessingError::InvalidRequest)?;
    if job.limits != limits {
        return Err(ProcessingError::InvalidRequest);
    }
    let result = match image_codec::process_job(&directory, &job) {
        Ok(manifest) => JobResult::Success {
            manifest: Box::new(manifest),
        },
        Err(error) => JobResult::Failure { error },
    };
    let metadata = serde_json::to_vec(&result).map_err(|_| ProcessingError::InternalError)?;
    if metadata.len() as u64 > limits.result_metadata_bytes(job.recipe.outputs.len())? {
        return Err(ProcessingError::InternalError);
    }
    fs::write(directory.join("result.json"), metadata)
        .map_err(|_| ProcessingError::InternalError)?;
    Ok(())
}
