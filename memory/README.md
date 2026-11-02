# Verification milestones

- [2026-11-02: JPEG preview and resize optimization](2026-11-02-decode-resize-optimization.md): accurate defaults, bounded reduced decoding, exact area-filter comparisons, Astra review, and Windows checks.

- [2026-11-02: imgproxy benchmark harness](2026-11-02-imgproxy-benchmark-harness.md): fixed work units, resource profiles, measured CPU intervals, visual checks, and failure retention.

- [2026-11-02: version 0.3.0 publication](2026-11-02-version-0.3.0.md): immutable release tag, successful tag CI, published GHCR digest, and direct registry-image verification.

- [2026-11-02: Linux container verification](2026-11-02-linux-container-verification.md): Linux workspace and resource-test evidence, runtime profile checks, and accurate HTTP request counts.

- [2026-11-01: streaming image formats](2026-11-01-streaming-image-formats.md): broader input formats, preserved animation, no raw frame spool, Astra review, and local verification limits.

- [2026-10-17: version 0.2.1](2026-10-17-version-0.2.1.md): prepare a patch version for the writing changes after the published 0.2.0 tag.

- [2026-10-17: technical writing](2026-10-17-technical-writing.md): writing guide, revised documents and comments, and checks that preserve contracts and historical evidence.

- [2026-10-16: version 0.2.0](2026-10-16-version-0.2.0.md): align workspace, lockfile, OpenAPI and container release metadata after the 0.1.0 tag.

- [2026-10-16: independent Sharp benchmarks](2026-10-16-image-benchmarks.md): self-contained baseline, recorded local measurements and functional verification boundaries.

- [2026-10-16: operator budgets](2026-10-16-operator-budgets.md): configurable capacity, native/IPC fences and independent regression coverage.

- [2026-10-16: image service implementation](2026-10-16-image-service.md): template import, image contracts, native worker isolation, Astra review and local/container evidence.
- [2026-10-16: Linux delivery cancellation](2026-10-16-linux-delivery.md): reproduce the remote CI slow-consumer failure and distinguish abortive cancellation from normal response closure.
- [2026-10-16: GHCR container releases](2026-10-16-container-release.md): version-tag checks, exact tested-image publication and published-image deployment profile.

Use [the milestone template](TEMPLATE.md) and [the writing guide](../docs/writing.md) for later changes.
These records describe observed checks. Production capacity and deployment need separate validation.
