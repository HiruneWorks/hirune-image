# Changelog

## 0.4.0

- Keep accurate decoding as the default. Add optional `decode_mode=preview` for shared reduced JPEG IDCT decoding.
- Preserve original facts, oriented output geometry, and original resource checks in preview mode.
- Retain the accurate path for ICC, CMYK, other formats, and requests that require the full-resolution image.
- Cache bounded area-filter geometry across frames. Keep the source accumulation order and linear-light premultiplied-alpha policy.
- Use transfer lookup tables with inverse-transfer boundary fallback. Encode unchanged-size RGBA directly without cloning it.
- Add reproducible imgproxy service benchmarks with photo, alpha, animation, concurrency, origin traffic, and one/four/eight-output workloads.

## 0.3.0

- Generate every requested size as each display frame arrives. Remove the full RGBA frame spool.
- Add APNG, AVIF, HEIC/HEIF, BMP, single-page TIFF, JPEG XL, static SVG, ICO, QOI, and PNM input.
- Support SDR 16-bit PNG/APNG and CMYK JPEG color normalization.
- Preserve display frames, alpha, rational timing, finite/infinite loops, and single-frame animation containers.
- Bound the combined compressed mux size while encoding. Keep final full-output verification.
- Build the pinned AVIF decoder from hash-checked source archives. Add codec versions to capabilities.
- Breaking changes: accept only `animation_policy=preserve`, add the `animated` fact, and remove `spool_bytes`/`HIRUNE_SPOOL_BYTES`.
- Reject unsupported HEIF timed tracks, multipage TIFF, active SVG, HDR, and associated-alpha JPEG XL explicitly.

## 0.2.1

- Add a practical ASD-STE100 writing guide for specifications, runbooks, technical documents, and code comments.
- Clarify existing documents and OpenAPI descriptions. Preserve API behavior, resource limits, commands, and historical measurements.

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
