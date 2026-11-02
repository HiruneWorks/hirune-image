# 2026-11-02 — imgproxy benchmark harness

## Changes

Add a Docker benchmark for complete image-variant work units.
The harness compares existing Hirune and imgproxy OSS v4.0.20 images.
It pins the imgproxy registry digest and records both service image identities.
It gives both services the same CPU, memory, filesystem, user, and process-count limits.
The Python/Pillow client and in-memory fixture origin run in a separate container.

The inputs include a fixed NASA photo, progressive and EXIF variants, transparent PNG, and a 60-frame animation.
Output side limits are 384, 192, 96, and 48 pixels.
Each input requires actual resizing. The report retains source hashes and generator provenance.
One Hirune POST produces all sizes. imgproxy uses one URL per size.
The report records attempted HTTP requests and origin download counts and bytes.

The accurate imgproxy profile enables linear colorspace and disables shrink-on-load.
The preview profile retains linear colorspace but permits shrink-on-load.
These are explicit comparison profiles, not claims about imgproxy's complete default settings.
Filters, codecs, verification work, metadata, and HTTP contracts still differ.

## Decisions and constraints

The user selected accurate decoding as the default for the planned optimization.
JPEG preview decoding must be an explicit request option.
This harness commit does not implement that option.
Watermarking and unrelated image editing remain outside this task.

Astra reviewed the experiment design and first implementation.
The review led to a CPU batch-completion handshake, failed-scenario retention, alternating provider order, and stronger visual checks.
Validation checks alpha regions, orientation markers, all animation frame markers, delays, loops, and full output decoding.
It runs outside the timed batch and after the host reads service CPU.
Hirune's production-side verification remains part of its measured work.
The benchmark records imgproxy's generated EXIF without weakening Hirune's metadata-free contract.

## Validation

- Python syntax checks passed.
- All 20 Docker smoke scenarios passed: five inputs, one or four outputs, and two providers.
- Four additional smoke scenarios passed for V7's eight-width wiki recipe with photo and animation inputs.
- Smoke runs used one measured request per scenario. They validate the harness, not performance conclusions.
- An intentional request for the unsupported preview option against Hirune 0.3.0 failed with HTTP 400.
- The harness retained that scenario as failed at the warmup stage and marked the report incomplete.
- Temporary scenario containers and networks were removed.

## Remaining work

Run repeated baseline measurements before changing codec code.
Implement the default-accurate resizer optimization and opt-in JPEG preview decoder.
Repeat the same workloads and obtain Astra implementation review.
Remote CI, release publication, deployment, and caller integration did not run for this harness milestone.
