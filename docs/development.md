# Development

Install Rust 1.98.1, a C toolchain, `just`, Python 3, and Docker.
Use MSVC as the C toolchain on Windows.
Python runs the independent response verifier. Docker runs the Linux container checks.
Run the commands below from the repository root.

| Command | Purpose |
| --- | --- |
| `just build` | Build the API and worker. |
| `just api` | Build the worker, then start the API. |
| `just check` | Check formatting, clippy, workspace tests, and generated contracts. |
| `just test-api` | Test image contracts over actual TCP HTTP requests. |
| `just test-process` | Test overload, upload timeout, child exit, cancellation, and cleanup. |
| `just contracts` | Regenerate schemas and OpenAPI together. |
| `just test-container` | Build and check the Linux image with production restrictions. |
| `just test-resources` | On Linux, test address-space and CPU exhaustion, child exit, and the next request. |
| `just bench` | Run release-mode Criterion component benchmarks. |
| `just bench-services` | Compare Hirune and Sharp with the same Docker resource limits. |

Configure environment variables explicitly. See the [.envs examples](../.envs/README.md).
Git ignores directories that contain actual environment settings.
The executables do not search for a root dotenv file.
If a long Windows path causes native C compilation to fail, set `CARGO_TARGET_DIR` to a shorter path for this task.

Tests use the production router and supervisor.
The [fixture guide](../tests/fixtures/README.md) records source details and expected timelines.
Keep container checks, remote CI, deployment, and caller or storage integration as separate results.
Development defaults do not establish production capacity.

See [benchmarks](benchmarks.md) for measurement boundaries and [operator budgets](operations.md) for configuration.
ICC pixel tests use a committed reference generated independently with Pillow and LittleCMS.
Normal tests do not need Pillow.
Resource tests compile a helper executable. The production image excludes that executable.
CI runs the Linux resource tests with the workspace suite.
