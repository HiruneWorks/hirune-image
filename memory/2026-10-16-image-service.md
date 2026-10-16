# 2026-10-16 — Image service implementation

## Changes

Imported the complete tracked Hirune Axum Kit source from `f93f0b6fbcafcfa884510fa5c581e60fb15978f5` in local commit `f7872c4`.
Adapted its layout and conventions for the image microservice. Reference repositories stayed unchanged.
Removed unrelated authentication, Cedar, database, queue, mail, and storage features.

Added strict multipart requests and one to four WebP outputs encoded in sequence.
Added source, recipe, and output SHA-256 bindings, manifests, fixed error envelopes, and capabilities.
Generated JSON Schema and OpenAPI, and added an independent Python response verifier.
The application layer has no HTTP dependency. Native bindings stay in the codec crate.
Actual HTTP tests use the production router and child worker.

## Decisions and constraints

Decode the source pixels once into temporary RGBA files for that request. Enforce a byte limit on those files.
Encode outputs in sequence, then immediately delete the source and RGBA files.
Keep completed outputs and their admission lease until delivery or cancellation.
The supplied container uses tmpfs. Those temporary files consume memory and count toward the container's cgroup limit.

Preserve displayed animation frames, delays, and repetitions with complete canvas WebP mux frames.
Do not combine identical frames.
Apply supported RGB ICC conversion and EXIF orientation before removing metadata.
Use an area downsampling filter in linear light with premultiplied alpha. Do not enlarge the image.
Reject unsupported color and animation cases explicitly.

Supervise one native child for each admitted request.
Retain the admission permit through file operations, cancellation, child exit, and cleanup.
Set limits for upload, canvas, frames, total decoded pixels and bytes, working memory, temporary files, duration, and outputs.
Apply request deadlines too.
Linux children apply address-space and CPU limits and `no-new-privileges`.
Windows enforces deadlines but does not provide the Linux native memory limit.

Astra reviewed the source.
The changes addressed multipart epilogues, response buffer ownership, file operation deadlines, child exit, and low-bit PNG support.
They also addressed decoder-start metrics, OpenAPI references, and ICC declared-length checks.
Direct libwebp configuration controls effort and output writes with byte limits.
Native decode errors are not treated as normal end of stream.
Regression tests check exact static and animation output limits, including mux size accounting.

## Validation

- Local Windows `just check` passed formatting, all-target clippy with warnings denied, 24 tests, and generated contract drift checks.
  The tests covered 13 HTTP image cases, six process cases, three contract/budget/decode-once cases, and two codec/resize cases.
- Final local checks included Python compilation, fixture hashes, Compose configuration, Markdown links, and Git whitespace.
- `python scripts/verify_container.py` passed eight image HTTP cases against the Linux release image on an internal Docker network.
  The container used UID 65532, a read-only root filesystem, dropped capabilities, `no-new-privileges`, limited tmpfs, and 1 GiB of memory.
  Checks confirmed an empty temporary directory after requests, worker capabilities, and no OOM termination.
- Final container build ID: `development:14a4a7f5b5a5b3fcd5ae92fd44a9f51f07e32bd2128fb149421121039bc04d5a`. Runtime libwebp encoder/decoder/mux/demux: 1.5.0; image: 0.25.10; gif: 0.14.2; moxcms: 0.8.1.
- The container memory peak for these small fixtures was 6,512,640 bytes.
  This short functional check did not measure worst-case memory or throughput.

## Status at this milestone

Remote CI, publication, deployment, and caller storage/publication integration had not run.
Production capacity still needed representative large-image and concurrency measurements under the intended Linux resource profile.
The full 24-test suite had run on Windows. Linux checks covered the production container's eight image HTTP cases.
