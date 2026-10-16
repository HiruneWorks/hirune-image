# Changelog

## 0.1.0

- Add a stateless Rust/Axum image service with typed recipes and verified multipart WebP variants.
- Support inside resize, alpha-aware linear-light filtering, RGB ICC conversion, EXIF orientation, static/animated lossy/lossless WebP, GIF loop conversion, and explicit unsupported-input failures.
- Isolate native codecs in request children with bounded admission, budgets, deadlines, TCP delivery cancellation, kill/reap, and scratch cleanup.
- Provide generated schemas/OpenAPI, examples, an independent response verifier, actual HTTP/process tests, and a private non-root read-only container profile.
