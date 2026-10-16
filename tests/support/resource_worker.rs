#![forbid(unsafe_code)]
// This executable supports tests. The production image's explicit COPY list excludes it.
use application::images::{JobInput, JobResult, Limits};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let directory = std::path::Path::new(&args[2]);
    let limits: Limits = serde_json::from_str(&args[4]).unwrap();
    limits.validate().unwrap();
    image_codec::sandbox::apply_resource_limits(
        limits.child_memory_bytes,
        limits.child_cpu_seconds,
    )
    .unwrap();
    let job: JobInput =
        serde_json::from_slice(&std::fs::read(directory.join("job.json")).unwrap()).unwrap();
    std::fs::write(directory.join("child.pid"), std::process::id().to_string()).unwrap();
    // Wait briefly so the parent can read the PID before the child exhausts its limit.
    if job.recipe.outputs[0].id == "memory" || job.recipe.outputs[0].id == "cpu" {
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    match job.recipe.outputs[0].id.as_str() {
        "memory" => {
            let mut memory = Vec::<u8>::new();
            if memory
                .try_reserve_exact(limits.child_memory_bytes as usize + 1)
                .is_err()
            {
                std::process::exit(77);
            }
            // If allocation succeeds, return a normal response. The test must then fail.
            write_result(directory, &job);
        }
        "cpu" => loop {
            std::hint::black_box(123u64.wrapping_mul(456));
        },
        _ => write_result(directory, &job),
    }
}
fn write_result(directory: &std::path::Path, job: &JobInput) {
    let result = match image_codec::process_job(directory, job) {
        Ok(manifest) => JobResult::Success {
            manifest: Box::new(manifest),
        },
        Err(error) => JobResult::Failure { error },
    };
    std::fs::write(
        directory.join("result.json"),
        serde_json::to_vec(&result).unwrap(),
    )
    .unwrap();
}
