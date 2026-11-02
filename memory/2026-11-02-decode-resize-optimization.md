# 2026-11-02 — JPEG preview and area-resize optimization

## Changes

Prepare version 0.4.0 with optional `decode_mode=preview`. Omitted mode remains `accurate`.
Preview selects one shared reduced JPEG IDCT scale for all requested outputs.
Original oriented geometry determines output sizes. Original source facts and admission budgets remain authoritative.
ICC, CMYK, non-JPEG input, and requests without a useful reduction retain accurate decoding.
The preview decoder uses pinned `jpeg-decoder` 0.3.2 without Rayon.

Cache area-filter axis spans within the remaining source working plan.
Keep source-Y/source-X accumulation order and premultiplied linear-light color calculations.
Use transfer tables with the previous inverse transfer near quantization boundaries.
Pass unchanged-size RGBA directly to the encoder without a clone.
Generate all sizes as each animation frame arrives. Keep the existing compressed mux and output verification.

## Decisions and constraints

The user selected accurate defaults with an explicit preview option.
Preview can change pixels, including when the output uses lossless encoding.
Keep version 1 manifest and capability field shapes unchanged. Update recipe schema and OpenAPI together.
Keep this service and its benchmark profiles independent of callers.
Do not add watermarking, URL input, storage, or application-specific policies.

Astra reviewed the implementation. The review found a progressive JPEG coefficient reservation gap.
Before pixel decoding, preview now reserves MCU-padded full-resolution coefficients and reduced pixel buffers.
It falls back to accurate decoding if that plan does not fit. The frame callback passes the reservation to the cache planner.
Astra rechecked that correction and found no further blocking issue.
These reservations remain coarse working plans. Linux child address-space and CPU fences remain the hard process limits.

## Validation

- `cargo xtask check` passed on Windows: formatting, Clippy with warnings denied, 59 workspace tests, and generated-contract checks.
- Resize tests compare cached and uncached pixels against the previous scalar algorithm over 400 deterministic odd-size and alpha cases.
- Transfer tests check one million values and half-code boundary neighbors against the previous inverse transfer.
- Preview tests cover baseline/progressive JPEG, grayscale, thin images, all eight EXIF color-corner transforms, accurate fallbacks, source limits, and corrupt tails.
- Actual HTTP tests confirm omission defaults, accepted modes, unknown/duplicate rejection, geometry, and response verification.
- Fixture SHA-256 checks and release metadata validation for `v0.4.0` passed.

## Remaining work

Build and verify the Linux candidate image. Repeat the fixed imgproxy workloads and record the results.
Remote CI, version publication, deployment, and caller integration have not run for this implementation milestone.
