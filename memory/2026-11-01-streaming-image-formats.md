# 2026-11-01 — Streaming image formats and animated variants

## Changes

Version 0.3.0 adds APNG, AVIF, HEIC/HEIF, BMP, single-page TIFF, JPEG XL, static SVG, ICO, QOI, and PNM input.
JPEG, PNG, GIF, and WebP remain supported.
PNG/APNG support SDR samples through 16 bits. JPEG supports CMYK profiles.
The worker converts supported color profiles before reducing sample precision.

The worker decodes one display frame and sends it to every requested output encoder.
It no longer writes raw RGBA frame files or rereads them for each size.
Compressed output muxes remain in memory under one combined byte budget.
The worker still verifies every complete encoded output before returning a response.

GIF, APNG, WebP, AVIF, and JPEG XL retain animation in every size.
The output retains complete composited frames, alpha, frame order, timing, and playback count.
Single-frame animation containers retain their duration and animation flag.
Fractional source timing uses cumulative rounding to integer milliseconds.
The worker rejects unrepresentable delays. It never drops frames to make an animation fit.

The AVIF adapter builds pinned libavif 1.3.0 and decoder-only libaom 3.13.1.
The build checks source archive hashes before extraction.
The repository includes upstream license and patent notices.
Committed synthetic fixtures let normal tests run without fixture-generation tools.
FFmpeg is used only by the optional fixture generator. It is not a service dependency.

## Decisions and constraints

Only `animation_policy=preserve` is valid in version 0.3.0.
The manifest adds the required `animated` fact.
The obsolete `spool_bytes` field and `HIRUNE_SPOOL_BYTES` setting are removed.
Output remains WebP. Original-file storage and serving belong to the caller.
Static SVG is rasterized when the caller requests a WebP variant.

The worker rejects HEIF timed tracks, multipage TIFF, active SVG, explicit HDR, and associated-alpha JPEG XL.
It returns a typed error instead of a static first frame.
SVG cannot load external resources. Embedded supported static images use the normal color path.
SVG uses an 8-bit premultiplied surface; very low alpha can reduce hidden RGB precision.
See the [behavior contract](../docs/behavior/image-processing.md) for exact format and color boundaries.

Cumulative decoded-work limits remain separate from live-pixel limits.
Native codecs can retain reference state. The pixel budget does not measure all native allocations.
The worker keeps its existing process isolation, admission lifetime, deadlines, cancellation, and cleanup.
The new architecture has no production capacity measurements yet.

## Validation

Astra reviewed the implementation in several read-only passes.
The review led to explicit single-frame animation handling, bounded WebP mux assembly, and stricter source validation.
It also led to uncropped HEIC geometry checks and a budget for each decoded tile reference, including repeated references.
The final review found no remaining blocking issues in the reviewed implementation.

Local Windows `cargo xtask check` passed formatting, Clippy with warnings denied, all workspace tests, and generated-contract drift checks.
The 14 format tests cover real encoded modern-format fixtures, 16-bit APNG composition, hidden APNG posters, fractional timing, alpha, loops, all output sizes, late corruption, and combined output limits.
Regression tests also cover HTTP contracts, independent response verification, process cancellation, timeout cleanup, and absence of raw frame files.
The independent `cargo run --locked --bin contracts -- --check` also passed.

Linux/container tests did not run.
Docker Engine was unavailable. Starting WSL Ubuntu failed with `Wsl/Service/CreateInstance/MountDisk/HCS/E_ACCESSDENIED` while attaching `D:\WSL\Ubuntu\ext4.vhdx`.
Linux address-space and CPU enforcement still require execution on a working Linux host.
Remote CI did not run. No image was published and no service was deployed.
Caller integration did not run.

## Remaining work

Run the container verifier and Linux resource tests on a working Linux host.
Measure current memory and throughput before changing production capacity settings.
Update callers for the version 0.3.0 contract changes before rollout.
