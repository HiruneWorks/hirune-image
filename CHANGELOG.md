# Changelog

## 0.2.0

- Let operators raise or lower resource budgets. Report fixed codec limits separately.
- Apply native resource limits before reading job metadata. Size IPC limits from recipe bytes and output count.
- Add independent ICC and resized-animation reference pixels. Test every frame delay and larger operator budgets through HTTP and schemas.
- Test Linux address-space and CPU exhaustion, child exit, and request recovery. Check fixture hashes, bounded header mutations, and extreme geometry.
- Add Criterion component benchmarks and an independent Node/Sharp Docker comparison.
- Record validated responses, latency, throughput, output bytes, and cgroup resources. State workload and filter differences with the results.

## 0.1.0

- Add a stateless Rust/Axum image service with typed recipes and verified multipart WebP outputs.
- Support `inside` resize without enlargement. Use an area filter in linear light with premultiplied alpha.
- Apply RGB ICC conversion and EXIF orientation. Support static and animated lossy or lossless WebP, with GIF loop conversion.
- Reject unsupported input explicitly.
- Run native codecs in a child for each request. Enforce admission, resource budgets, deadlines, and TCP delivery cancellation.
- Kill the child and wait for its exit before releasing resources. Remove temporary files after each request.
- Add generated schemas, OpenAPI, examples, an independent response verifier, and actual HTTP and process tests.
- Add a private container profile with a non-root user and read-only root filesystem.
- On matching version tags, publish the exact tested Linux image to GHCR.
- Keep code and container verification separate from the job that writes the package. Record commit identity, OCI labels, and the image digest.
- Pull the published image in production Compose. Keep source builds in the local override.
- Report the active moxcms 0.9.1 color engine after the dependency update.
