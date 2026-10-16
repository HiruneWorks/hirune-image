use application::{
    error::ProcessingError,
    images::{JobInput, JobResult},
};
use std::{fs, path::PathBuf};
fn main() {
    if run().is_err() {
        std::process::exit(1)
    }
}
fn run() -> Result<(), ProcessingError> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() == 1 && arguments[0] == "--probe" {
        // Probe both modes and the actual native libraries before accepting traffic.
        image_codec::sandbox::apply_resource_limits(768 << 20, 5)?;
        print!(
            "{}",
            serde_json::to_string(&image_codec::probe()?)
                .map_err(|_| ProcessingError::InternalError)?
        );
        return Ok(());
    }
    if arguments.len() != 2 || arguments[0] != "--job" {
        return Err(ProcessingError::InvalidRequest);
    }
    let directory = PathBuf::from(&arguments[1]);
    let path = directory.join("job.json");
    if fs::metadata(&path)
        .map_err(|_| ProcessingError::InvalidRequest)?
        .len()
        > 64 << 10
    {
        return Err(ProcessingError::InvalidRequest);
    }
    let job: JobInput =
        serde_json::from_slice(&fs::read(path).map_err(|_| ProcessingError::InvalidRequest)?)
            .map_err(|_| ProcessingError::InvalidRequest)?;
    job.limits.validate()?;
    image_codec::sandbox::apply_resource_limits(
        job.limits.child_memory_bytes,
        job.limits.deadline_ms.div_ceil(1000),
    )?;
    let result = match image_codec::process_job(&directory, &job) {
        Ok(manifest) => JobResult::Success {
            manifest: Box::new(manifest),
        },
        Err(error) => JobResult::Failure { error },
    };
    fs::write(
        directory.join("result.json"),
        serde_json::to_vec(&result).map_err(|_| ProcessingError::InternalError)?,
    )
    .map_err(|_| ProcessingError::InternalError)?;
    Ok(())
}
