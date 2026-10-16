#[cfg(target_os = "linux")]
use application::error::ProcessingError as Error;
use application::error::Result;

pub fn apply_resource_limits(memory_bytes: u64, cpu_seconds: u64) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        // The child has not started Tokio or threads; restrict its own process only.
        let memory = libc::rlimit {
            rlim_cur: memory_bytes,
            rlim_max: memory_bytes,
        };
        let cpu = libc::rlimit {
            rlim_cur: cpu_seconds,
            rlim_max: cpu_seconds + 1,
        };
        let core = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if unsafe { libc::setrlimit(libc::RLIMIT_AS, &memory) } != 0
            || unsafe { libc::setrlimit(libc::RLIMIT_CPU, &cpu) } != 0
            || unsafe { libc::setrlimit(libc::RLIMIT_CORE, &core) } != 0
            || unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0
        {
            return Err(Error::ProcessorUnavailable);
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (memory_bytes, cpu_seconds);
    }
    Ok(())
}
