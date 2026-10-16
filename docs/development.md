# Development

Install Rust 1.98.1, a C toolchain (MSVC on Windows), just, Python 3 for independent response verification, and Docker for container acceptance.

- just build: build API and worker.
- just api: run the API after building the worker.
- just check: format check, clippy, workspace tests, contract drift.
- just test-api: actual TCP HTTP and image contracts.
- just test-process: overload, upload timeout, child kill/reap, cancellation and cleanup.
- just contracts: regenerate schemas and OpenAPI together.
- just test-container: build and verify the non-root read-only Linux image.
- just test-resources: Linux native AS/CPU exhaustion, reap and next-request recovery.
- just bench: release-mode Criterion component benchmarks.
- just bench-services: compare Hirune and the independent Sharp baseline under matching Docker resources.

Environment variables are explicit; see [.envs examples](../.envs/README.md). Actual environment directories are ignored. No root dotenv discovery runs inside the executables. If native C compilation fails on a long Windows path, set CARGO_TARGET_DIR to a shorter task-specific path.

Tests use the same router and supervisor as production. Fixture provenance and expected timelines live in [fixtures](../tests/fixtures/README.md). Container validation, remote CI, deployment and caller/storage integration are separate results. Development defaults are not capacity measurements.

See [benchmarks](benchmarks.md) for measurement boundaries and [operator budgets](operations.md) for configuration. ICC pixel tests use a committed independent Pillow/LittleCMS golden. Normal tests need no Pillow installation. Resource tests compile a test helper executable that is excluded from the runtime image. CI runs the Linux resource tests as part of the workspace suite.
