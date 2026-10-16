# 2026-10-16 — Image service implementation

## Changes

Imported the complete tracked Hirune Axum Kit source from f93f0b6fbcafcfa884510fa5c581e60fb15978f5 in local commit f7872c4, then adapted its layout and conventions for the image microservice. Reference repositories were left unchanged. Removed unrelated authentication, Cedar, database, queue, mail and storage features.

Implemented strict multipart requests, one to four sequential WebP outputs, source/recipe/output SHA-256 binding, manifests, fixed error envelopes, capabilities, generated JSON Schema/OpenAPI and an independent Python response verifier. The application layer remains HTTP-free; native bindings are confined to the codec crate. Real HTTP tests use the production router and child worker.

## Decisions and constraints

Decode the source pixels once into bounded request-local RGBA scratch files. Encode outputs sequentially, then immediately delete the source and RGBA files. Completed outputs and their admission lease remain until delivery or cancellation. Scratch files are local and ephemeral; the supplied container uses tmpfs, which consumes memory and counts toward its cgroup limit.

Preserve displayed animation frames, delays and repetitions using full-canvas WebP mux frames without coalescing. Apply supported RGB ICC and EXIF orientation before stripping metadata. Use linear-light premultiplied-alpha area downsampling without enlargement. Explicitly reject unsupported color/animation cases rather than silently changing them.

Supervise one native child per admitted request. Hold admission through filesystem work, cancellation, child kill/reap and cleanup. Bound upload, canvas, frames, cumulative decoded pixels/bytes, working memory, scratch, duration, output count/bytes and deadlines. Linux children apply address-space/CPU limits and no-new-privileges. Windows enforces deadlines but does not provide the Linux native memory fence.

Astra performed source reviews. Addressed multipart epilogues, response buffer ownership, filesystem deadlines, child reaping, low-bit PNG support, actual decoder-start metrics, OpenAPI references and ICC declared-length validation. Direct libwebp configuration controls effort and bounded output writing; native decode errors are never treated as normal end-of-stream. Exact static and animation output-cap regressions cover the mux accounting.

## Validation

- Local Windows `just check`: passed formatting, all-target clippy with warnings denied, 24 tests and generated contract drift checks. The tests include 13 actual HTTP image cases, six lifecycle/process cases, three contract/budget/decode-once cases and two codec/resize unit tests.
- Python script compilation, fixture checksums, Compose configuration, Markdown links and Git whitespace checks are included in final local housekeeping.
- `python scripts/verify_container.py`: passed eight real image HTTP cases against the Linux release image on an internal Docker network, with UID 65532, a read-only root filesystem, dropped capabilities, no-new-privileges, bounded tmpfs and a 1 GiB memory limit. Checked empty scratch after requests, worker capabilities and absence of OOM termination.
- Final container build ID: `development:14a4a7f5b5a5b3fcd5ae92fd44a9f51f07e32bd2128fb149421121039bc04d5a`. Runtime libwebp encoder/decoder/mux/demux: 1.5.0; image: 0.25.10; gif: 0.14.2; moxcms: 0.8.1.
- Reported container memory peak for the small acceptance fixtures: 6,512,640 bytes. This is smoke-test evidence, not worst-case memory sizing or a throughput measurement.

## Remaining work

Remote CI, publication, deployment and caller storage/publication integration were not run. Production capacity needs representative large-image/concurrency measurements under the intended Linux resource profile. The full 24-test suite was run locally on Windows; Linux validation covered the production container's eight image HTTP cases.
