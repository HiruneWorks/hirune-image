use application::images::Limits;
use std::{net::SocketAddr, path::PathBuf};
#[derive(Clone)]
pub struct Settings {
    pub bind: SocketAddr,
    pub limits: Limits,
    pub worker: PathBuf,
    pub temp_root: PathBuf,
}
impl Settings {
    pub fn from_environment() -> anyhow::Result<Self> {
        let mut limits = Limits::default();
        macro_rules! setting {
            ($field:ident,$name:literal) => {
                if let Ok(value) = std::env::var($name) {
                    limits.$field = value
                        .parse()
                        .map_err(|_| anyhow::anyhow!(concat!("invalid ", $name)))?
                }
            };
        }
        setting!(source_bytes, "HIRUNE_SOURCE_BYTES");
        setting!(request_bytes, "HIRUNE_REQUEST_BYTES");
        setting!(recipe_bytes, "HIRUNE_RECIPE_BYTES");
        setting!(input_dimension, "HIRUNE_INPUT_DIMENSION");
        setting!(canvas_pixels, "HIRUNE_CANVAS_PIXELS");
        setting!(frames, "HIRUNE_FRAMES");
        setting!(total_frame_pixels, "HIRUNE_TOTAL_FRAME_PIXELS");
        setting!(decoded_bytes, "HIRUNE_DECODED_BYTES");
        setting!(working_bytes, "HIRUNE_WORKING_BYTES");
        setting!(spool_bytes, "HIRUNE_SPOOL_BYTES");
        setting!(duration_ms, "HIRUNE_DURATION_MS");
        setting!(outputs, "HIRUNE_OUTPUTS");
        setting!(output_bytes, "HIRUNE_OUTPUT_BYTES");
        setting!(output_dimension, "HIRUNE_OUTPUT_DIMENSION");
        setting!(concurrency, "HIRUNE_CONCURRENCY");
        setting!(deadline_ms, "HIRUNE_DEADLINE_MS");
        setting!(idle_ms, "HIRUNE_IDLE_MS");
        setting!(child_memory_bytes, "HIRUNE_CHILD_MEMORY_BYTES");
        limits.validate()?;
        let worker = std::env::var_os("HIRUNE_WORKER")
            .map(PathBuf::from)
            .unwrap_or(std::env::current_exe()?.with_file_name(if cfg!(windows) {
                "image_worker.exe"
            } else {
                "image_worker"
            }));
        let temp_root = std::env::var_os("HIRUNE_TEMP_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        Ok(Self {
            bind: std::env::var("HIRUNE_BIND")
                .unwrap_or_else(|_| "0.0.0.0:8080".into())
                .parse()?,
            limits,
            worker,
            temp_root,
        })
    }
}
