# Development

Install Rust 1.98.1, a C/C++ toolchain, CMake, NASM, `just`, Python 3, and Docker.
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

## Native decoder build

The AVIF adapter builds libavif 1.3.0 and decoder-only libaom 3.13.1 from fixed source archives.
The build checks both SHA-256 values before extraction. It caches archives under `target/native-sources`.
The first build needs network access. Later builds can reuse that cache.
The local adapter supports NASM 2 and 3 and stages MSVC multi-configuration outputs explicitly.
See [adapter provenance](../crates/image-codec/native/avif-sys/README.md).
HEIC, SVG, JPEG XL, PNG/APNG, TIFF, and the other input wrappers use Rust libraries.
All native bindings remain under `crates/image-codec`.

The new [format fixtures](../tests/fixtures/formats-provenance.json) are synthetic and committed.
Normal tests do not need Pillow, pillow-heif, or FFmpeg.
Only `scripts/generate_format_fixtures.py` uses those tools to regenerate the encoded references.
The runtime service uses no FFmpeg process.
