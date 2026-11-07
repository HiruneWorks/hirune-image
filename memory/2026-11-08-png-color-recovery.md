# 2026-11-08 — PNG color recovery and version 0.4.1

## Changes

Fix issue 12. Accept standard sRGB-compatible `gAMA/cHRM` values without `sRGB` or ICC.
Separate PNG container inspection from PNG color selection.
Convert calibrated SDR gamma, chromaticities, and white points with an in-memory CMS profile.
Apply the profile to static PNG and APNG before 16-bit quantization. Preserve alpha.
Support full-range RGB SDR `cICP` and select color declarations in their documented priority order.
Recover from bad CRCs in known metadata that the output discards.
Add independent LittleCMS 2.16 D50 reference pixels and three PNG container regressions.
Prepare workspace version 0.4.1, pipeline 4, OpenAPI, capability policy, changelog, and deployment examples.

## Decisions and constraints

Keep significant CRC checks, chunk bounds, selected color validation, animation preservation, and resource budgets.
Reject exactly collinear primaries with integer arithmetic before the CMS converts coordinates to floating point.
A valid higher-priority declaration can supersede a damaged lower-priority color description.
Ignore zero gamma. Use documented sRGB defaults for missing gamma or chromaticities.
Keep the existing APNG 16-bit encoded-sample SOURCE/OVER composition. Normalize the composited displays afterward.
Linear-light processing still applies to resizing. This patch does not introduce linear-light APNG compositing.
Keep unsupported HDR and narrow-range PNG CICP explicit. Do not change JPEG or GIF input policy.
The user authorized push, merge, and version publication. Use unsigned commits and an unsigned annotated tag.

## Validation

- Windows: `just check` passed with 68 tests, warnings-denied Clippy, formatting, and generated contract checks at version 0.4.1. Nine tests cover PNG HTTP/real-worker regressions.
- Independent color reference: native LittleCMS 2.16 generated D50 pixels in a disposable Linux container. Fixture hashes passed review.
- Astra reviewed color priority, strict significant CRCs, integer degeneracy, APNG composition, fixtures, and contract wording. All blocking findings were corrected.
- The first full check found a stale fixture README hash after the documentation edit. Update the hash and rerun the full check successfully.
- The original 15-case HTTP reproduction now accepts all 12 recoverable/valid cases. Pixel CRC errors, trailing bytes, and missing IEND still return 422.
- The local Linux candidate passed both default and large container profiles. Each profile checked 28 processing requests and 56 outputs, including the three new PNG fixtures. The checks confirmed the build identity, non-root user, restrictions, cleanup, and no OOM kill.
- Release metadata accepted `v0.4.1`. The deployment Compose example passed configuration validation.
- Remote CI, main integration, and version publication are pending.
- Production deployment and caller integration were not performed.

## Remaining work

Finish remote CI, main integration, and immutable version publication.
Verify the published image separately from the local candidate.
