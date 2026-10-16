# Requirements

Hirune Image is a standalone Rust/Axum image microservice adapted from a complete copy of Hirune Axum Kit. Preserve responsibility-based layout, explicit types, thin handlers, documentation, and test conventions.

Accept bytes and a bounded recipe; return validated WebP variants and technical facts synchronously. Callers own authorization, durable retries, publication, and storage. No database, Cedar, accounts, mail, persistent queue, URL fetching, file-path API, video, or FFmpeg.

Inputs: JPEG, static 8-bit PNG, GIF, static/animated WebP. Outputs: lossy/lossless WebP; inside resize without enlargement; one to four variants; preserve/first_frame/reject policies. Apply EXIF orientation and supported RGB ICC before stripping metadata. Explicitly reject unsupported color information.

The [behavior contract](behavior/image-processing.md) fixes wire behavior, limits, and exclusions. Development budgets are not measured production capacity. Local, container, remote CI, and caller integration have separate acceptance boundaries.
