# 2026-10-16 — GHCR container releases

## Changes

Adapted the release approach from smol-media-processor.
The workflow checks the code, builds one Linux candidate, and tests its HTTP behavior under production restrictions.
It transfers and publishes that same image.
The existing CI workflow now has verify, container, and publish jobs.
Only a pushed `v*` tag can start publication.
Branch, pull-request, and manual checks do not publish.

The release tag must exactly match `v` plus the workspace package version.
Image names use lowercase letters. Version strings must be valid Docker tags.
The candidate has the Git commit build ID and OCI source, revision, version, and license labels.
Acceptance checks the advertised build ID.
Before tagging and pushing, publication checks the loaded image ID. It does not rebuild the image.
Only the publication job has `packages: write`. It uses the automatic workflow token.
The workflow records the registry digest.

At this milestone, production Compose pulled `ghcr.io/hiruneworks/hirune-image:0.1.0` by default.
Operators can select another version or digest with `HIRUNE_IMAGE`.
The local Compose override builds from source with the same runtime restrictions.
Updated the release and deployment documents.
Corrected the reported direct dependency to moxcms 0.9.1 after the user's dependency update.

## Decisions and constraints

Publish only `linux/amd64`. Do not publish a `latest` alias.
Allow prerelease versions when they match the workspace version and Docker tag syntax.
Reject Cargo `+` build metadata.
Registry version tags can change. Do not move or reuse a release tag.
Use the workflow's digest to identify the exact image for deployment.

Preserved the user's dependency and base-image updates.
The project toolchain remains pinned by rust-toolchain.toml. The compiler baseline did not change.
This work added no database, persistent storage, application credentials, or host deployment.

Astra reviewed the reference workflow and this change.
The separate container runner did not have just installed.
Fixed that job by invoking the Python acceptance script directly.
The reviewer found no other material blockers in tag validation, transfer identity, permissions, or Compose configuration.

## Validation

- Windows `just check`: passed all 24 tests, formatting, warnings-denied clippy, and generated-contract drift checks.
- `actionlint .github/workflows/ci.yml`: passed.
- Python script compilation: passed.
- Release metadata CLI: passed five checks for branch metadata, valid v0.1.0, a mismatched version, a missing v prefix, and trailing newline rejection.
- Production and local-override Compose configurations: passed `docker compose ... config --quiet`.
- Local Docker save/load/tag round trip: preserved the existing image's ID. This checks transfer mechanics. CI separately builds and accepts the release candidate.
- Documentation links and Git whitespace are checked before committing. The corresponding GitHub Actions run and final integration report record remote candidate-build and acceptance evidence.

## Status at this milestone

Pipeline configuration did not create a release tag or publish a package.
The first version-tag run must verify registry authentication and publication.
Host deployment and production capacity need separate validation.
