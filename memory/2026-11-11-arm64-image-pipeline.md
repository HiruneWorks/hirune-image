# 2026-11-11 ARM64 image pipeline

## Objective

- Build Linux ARM64 images on x86 and publish tested AMD64/ARM64 indexes.

## User-Approved Scope

- Preserve existing codec behavior and versioned publication policy.
- Deployment hosts pull images instead of compiling application Rust.

## Implementation Status

- Completed: native-builder cross toolchain, explicit AVIF CMake configuration,
  native codec acceptance, archive verification and two-platform publisher.
- Partial: hosted checks for the final workflow integration.
- Not started: publication of a new semantic release.

## Major Changes

- Select explicit Rust and C/C++ targets. Route AVIF's direct CMake invocation
  through the ARM64 toolchain file and preserve native x86 configuration.
- Transfer the exact built image to a native ARM64 runner and verify runtime
  configuration and filesystem hashes. Local engine IDs are not compared
  across Docker storage implementations.
- Run existing default and large processing profiles on physical ARM64.
- Publication waits for both platform gates and pushes the tested images
  before creating the versioned index. It does not rebuild the ARM64 image.
- Deduplicate whole branch/PR CI runs. Do not cancel tag publications.

## Validation

- Cross-build and native default/large processing passed at
  `12de3db976462b7f9e8237dbda54dbe148b8bee2` (Actions run 42843181946).
- Four archive guards and three publisher guards passed locally. Modified
  workflows passed actionlint. Final-source hosted acceptance is pending.
- No new application release or running-service update is claimed here.

## Remaining Work

- Complete final hosted acceptance, merge and publish accepted images.

## Notes For Next Agent

- Keep published 0.4.1 tags immutable. Record any added architecture variant
  with its source revision, native acceptance and registry digest.
