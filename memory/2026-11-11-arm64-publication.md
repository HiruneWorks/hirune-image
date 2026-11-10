# 2026-11-11 ARM64 publication

## Objective

- Publish the accepted ARM64 build of existing 0.4.1 code without replacing
  the existing canonical release tag.

## User-Approved Scope

- Cross-compile on x86 CI, run real processing on physical ARM64, and supply
  prebuilt images for deployment operators. Keep public records independent
  of callers and deployments.

## Implementation Status

- Completed: exact-source hosted checks, PR #14 integration, accepted ARM64
  archive publication and registry readback.
- Partial: none for this architecture-variant publication.
- Not started: no new semantic release was requested by this record.

## Major Changes

- PR #14 merged as `b1ac96b6258c973d5d58f592f98af57e25fab9f4`.
- Accepted source head: `cd28f74b7397de02b536a2d1c3f03c6cc5d67fcd`.
- CI image revision: `ca28924f14213c3b43168755aaf19e9b02b77180`.
- Additional tag: `ghcr.io/hiruneworks/hirune-image:0.4.1-arm64-ca28924f1421`.
- Deployable digest: `ghcr.io/hiruneworks/hirune-image@sha256:3a2026131bc3fddc96c6c119f06f9e97d4dd2e4e33d42f990f8b747e57f1c1c3`.
- Exported archive SHA-256:
  `8aa9243a8f21817c7ac3af8333da5ca867f3f21c7c91ed21efcc288e625f41ae`.
- The existing `:0.4.1` remains at
  `sha256:5fb084963c8a394d66ae23d52ecc8b23a00d06dfdf1fcdfa90d4cc81c32fdb5e`.
- Correct release documentation to describe both tested platforms and portable
  configuration/layer verification.

## Validation

- [Actions run 42863275118](https://github.com/HiruneWorks/hirune-image/actions/runs/42863275118)
  passed code checks, the AMD64 container, ARM64 cross-build and native ARM64
  default/large processing profiles. Automatic branch publication was skipped.
- Runtime source paths and locked dependencies match `v0.4.1` exactly.
- The operator loaded the accepted archive and verified its configuration and
  six filesystem layers before pushing the separate architecture tag.
- Pulled the published digest and compared platform, runtime configuration and
  filesystem layers with the accepted image. All values matched.
- A physical ARM64 deployment node pulled the same digest and completed a
  temporary container execution. This record does not claim service rollout.

## Remaining Work

- Future version tags use the integrated two-platform publisher. No release tag
  or existing canonical image was replaced by this publication.

## Notes For Next Agent

- Use immutable digests for deployment. Do not equate image publication with
  acceptance of an operator's running service.
