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
- The first full check found a stale fixture README hash after the documentation edit. The corrected hash passed the next full check.
- The original 15-case HTTP reproduction now accepts all 12 recoverable/valid cases. Pixel CRC errors, trailing bytes, and missing IEND still return 422.
- The local Linux candidate passed both default and large container profiles. Each profile checked 28 processing requests and 56 outputs, including the three new PNG fixtures. The checks confirmed the build identity, non-root user, restrictions, cleanup, and no OOM kill.
- Release metadata accepted `v0.4.1`. The deployment Compose example passed configuration validation.
- [Final PR CI](https://github.com/HiruneWorks/hirune-image/actions/runs/42398187945) passed at head `0f806a72892849928d5df986d39ffe4cede065a1`. It ran 69 Linux tests, including native memory/CPU exhaustion and recovery, and both 28-request container profiles. Branch-push CI also passed.
- Merge [PR 13](https://github.com/HiruneWorks/hirune-image/pull/13) into `main` as `f1ddb5b7c8428ee54cf9486513e44ca1a7b25f15`. The merged tree equals the reviewed head tree. Issue 12 closed through the merge.
- Create and push the unsigned annotated tag `v0.4.1`. Its tag object is `f3f29a17e54867c4e8a51a037f1b8a5f38f110b8`; its commit is the intended merge commit. Local and remote tag references match.
- [The tag workflow](https://github.com/HiruneWorks/hirune-image/actions/runs/42399181566) passed all three jobs: `verify` (`128483041667`), `container` (`128484193251`), and `publish` (`128485626975`). It ran 69 Linux tests and both runtime profiles, then transferred and published the exact tested image.
- Publish [GitHub release v0.4.1](https://github.com/HiruneWorks/hirune-image/releases/tag/v0.4.1).
- Pull the published GHCR image. Its registry digest matches the publication log. Registry configuration confirms `linux/amd64`, user `65532:65532`, entrypoint `/app/api`, version `0.4.1`, the intended merge revision, and the expected source label.
- The registry-pulled image passed both local runtime profiles by immutable digest. Each profile checked 28 requests and 56 WebP outputs. Both confirmed the expected build ID, restrictions, cleanup, and no OOM kill.
- Four additional published-image lossless requests processed standard PNG calibration, linear gamma, D50 calibration, and P3 ICC input. Pillow fully decoded each output. RGB error was at most one code value; alpha matched exactly. These fixture checks do not establish production capacity.
- Remove the temporary verification containers and networks.
- Production deployment and caller integration were not performed.

Published image: `ghcr.io/hiruneworks/hirune-image:0.4.1`.
Exact registry reference:

```text
ghcr.io/hiruneworks/hirune-image@sha256:5fb084963c8a394d66ae23d52ecc8b23a00d06dfdf1fcdfa90d4cc81c32fdb5e
```

The release build reports source fingerprint `80bf14a67b403ba7a2d3ff7c7c4297fd54d440091ddfb404a462174303378d3d`.
It matches the locally checked runtime sources and the reviewed PR container.

## Remaining work

Production deployment and caller integration require separate execution.
