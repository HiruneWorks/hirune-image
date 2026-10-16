# Requirements

Hirune Image is a standalone Rust/Axum image microservice.
It starts from a complete copy of Hirune Axum Kit.
Preserve the template's layout by responsibility, explicit types, thin handlers, documents, and test conventions.

Accept image bytes and a recipe within configured limits.
Return validated WebP outputs and technical facts in the same HTTP request.
The caller owns authorization, durable retries, publication, and storage.
The service has no database, Cedar, accounts, mail, or persistent queue.
It accepts no URL or file path as an image input. It does not process video or use FFmpeg.

Accept JPEG, static PNG up to 8-bit depth, GIF, and static or animated WebP.
Produce lossy or lossless WebP. Resize with `inside` fit and no enlargement.
The operator sets the maximum output count; the default is four.
Support the `preserve`, `first_frame`, and `reject` animation policies.
Apply EXIF orientation and supported RGB ICC profiles before removing metadata.
Reject unsupported color information explicitly.
Operators can raise resource budgets within codec and numeric representation limits.

The [behavior contract](behavior/image-processing.md) defines the wire format, limits, and exclusions.
Development defaults do not establish production capacity.
Record local tests, container tests, remote CI, and caller integration as separate results.
