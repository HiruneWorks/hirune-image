# 2026-10-16 — GHCR container releases

## Changes

Adapted the smol-media-processor release approach: code checks, one Linux candidate build, HTTP acceptance under production restrictions, artifact transfer and publication of the same image. Expanded the existing CI workflow into verify/container/publish jobs. Only pushed `v*` tags can run publication; branch, pull-request and manual checks do not publish.

The release tag must exactly match `v` plus the workspace package version. Image names are lowercased and version strings must be valid Docker tags. The candidate receives the Git commit build ID and OCI source/revision/version/license labels. Acceptance verifies the advertised build ID, and publication checks the loaded image ID before tagging and pushing without rebuilding. Only publication has `packages: write`, using the automatic workflow token. The workflow records the resulting registry digest.

Production Compose now pulls `ghcr.io/hiruneworks/hirune-image:0.1.0` by default, with a `HIRUNE_IMAGE` override for other versions or digests. The local Compose override retains source builds and the same runtime restrictions. Updated release/deployment documentation and reported the direct moxcms 0.9.1 dependency correctly after the user's dependency update.

## Decisions and constraints

Publish `linux/amd64` only, with no `latest` alias. Prerelease versions are allowed when they match the workspace version and Docker tag syntax; Cargo `+` build metadata is rejected. Version tags are registry names rather than immutable content identifiers: do not move/reuse release tags, and use the workflow's digest for exact deployment identity.

Preserved the user's dependency/base-image updates. The project toolchain remains pinned by rust-toolchain.toml; this change does not migrate the compiler baseline. No database, persistent storage, application credentials or host deployment were added.

Astra reviewed both the reference workflow and the concrete change. Corrected the separate container runner's missing just installation by invoking the Python acceptance script directly. The reviewer found no other material blockers in tag validation, transfer identity, permissions or Compose wiring.

## Validation

- Windows `just check`: passed all 24 tests, formatting, warnings-denied clippy and generated-contract drift checks.
- `actionlint .github/workflows/ci.yml`: passed.
- Python script compilation: passed. Release metadata CLI: five passing checks covering branch metadata, valid v0.1.0, mismatched version, missing v prefix and trailing newline rejection.
- Production and local-override Compose configurations: passed `docker compose ... config --quiet`.
- Local Docker save/load/tag round trip preserved the existing image's ID. This checks transfer mechanics; the current release candidate is separately built and accepted by CI.
- Documentation links and Git whitespace are checked before committing. Remote candidate-build/acceptance evidence is recorded by the corresponding GitHub Actions run and final integration report.

## Remaining work

No release tag or package publication was performed as part of configuring the pipeline. The first version-tag run is the registry authentication/publication acceptance boundary. Host deployment and production capacity validation remain separate.
