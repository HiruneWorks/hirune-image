# Environment settings

Use `.example/app.env` for the initial settings and `.example/large.env` for a larger profile.
For local development, copy the settings to `.local/app.env` and run `just`.
Git ignores that local file. In a deployment, inject the values explicitly.
The service needs no credentials.

Operators can raise or lower budgets within codec and numeric representation limits.
Invalid combinations prevent startup.
`/v1/capabilities` reports active budgets and fixed codec limits in separate objects.
See [operator budgets](../docs/operations.md) for memory, tmpfs, and container sizing.

## Runtime settings

| Variable | Purpose |
| --- | --- |
| `HIRUNE_BIND` | API listen address. |
| `HIRUNE_WORKER` | Path to the worker executable, controlled by the operator. |
| `HIRUNE_TEMP_ROOT` | Filesystem directory for temporary request data. |
| `RUST_LOG` | Logging configuration. |

## Request and image budgets

| Variables | Scope |
| --- | --- |
| `HIRUNE_SOURCE_BYTES`, `HIRUNE_REQUEST_BYTES`, `HIRUNE_RECIPE_BYTES` | Source, full request, and recipe byte limits. |
| `HIRUNE_INPUT_DIMENSION`, `HIRUNE_CANVAS_PIXELS` | Maximum source side length and canvas pixel count. |
| `HIRUNE_FRAMES`, `HIRUNE_TOTAL_FRAME_PIXELS` | Frame count and the sum of frame pixels. |
| `HIRUNE_DECODED_BYTES`, `HIRUNE_WORKING_BYTES`, `HIRUNE_SPOOL_BYTES` | Decoded bytes, working pixel plan, and temporary frame file bytes. |
| `HIRUNE_DURATION_MS` | Duration of one animation cycle. |
| `HIRUNE_OUTPUTS`, `HIRUNE_OUTPUT_BYTES`, `HIRUNE_OUTPUT_DIMENSION` | Output count, total encoded bytes, and maximum requested side length. |
| `HIRUNE_CONCURRENCY` | Maximum admitted requests. |
| `HIRUNE_DEADLINE_MS`, `HIRUNE_IDLE_MS` | Absolute wall deadline and idle timeout. |
| `HIRUNE_CHILD_MEMORY_BYTES` | Linux child address-space limit. |
| `HIRUNE_CHILD_CPU_SECONDS` | Linux child CPU time, independent of the wall deadline. |

## Container and build settings

`HIRUNE_CONTAINER_MEMORY`, `HIRUNE_TMPFS_SIZE`, `HIRUNE_CONTAINER_CPUS`, and `HIRUNE_CONTAINER_PIDS` configure the Compose container.

`HIRUNE_BUILD_ID` is set at compile time. It does not come from a caller or runtime request.
An ordinary local build uses `development` as that part of its build identity.
