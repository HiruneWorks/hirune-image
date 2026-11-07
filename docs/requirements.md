# Requirements

Hirune Image is a standalone Rust/Axum image microservice.
It starts from a complete copy of Hirune Axum Kit.
Preserve the template's layout by responsibility, explicit types, thin handlers, documents, and test conventions.

Accept image bytes and a recipe within configured limits.
Return validated WebP outputs and technical facts in the same HTTP request.
The caller owns authorization, durable retries, publication, and storage.
The service has no database, Cedar, accounts, mail, or persistent queue.
It accepts no URL or file path as an image input. It does not process video or use FFmpeg.
Watermarking is outside the service requirements.

Accept JPEG, PNG/APNG, GIF, WebP, AVIF, HEIC/HEIF, BMP, single-page TIFF, JPEG XL, static SVG, ICO, QOI, and PNM.
Support 8-bit and 16-bit SDR PNG/APNG and CMYK JPEG.
Support calibrated SDR PNG gamma/chromaticities and supported full-range RGB CICP declarations.
Reject unsupported animation or color features explicitly. Never return a static substitute for an animation.
Produce lossy or lossless WebP. Resize with `inside` fit and no enlargement.
Default to accurate full-resolution decoding. Permit reduced JPEG decoding only when the caller selects preview mode.
Retain original geometry and resource admission checks in preview mode. Never enlarge a reduced canvas to satisfy a requested output.
The operator sets the maximum output count; the default is four.
Require the `preserve` animation policy. Preserve every display frame, alpha, timing, and total playback count.
Generate all requested sizes as each source frame arrives. Do not store raw frame files.
Apply EXIF orientation and supported RGB ICC profiles before removing metadata.
Reject unsupported color information explicitly.
Operators can raise resource budgets within codec and numeric representation limits.

The [behavior contract](behavior/image-processing.md) defines the wire format, limits, and exclusions.
Development defaults do not establish production capacity.
Record local tests, container tests, remote CI, and caller integration as separate results.
