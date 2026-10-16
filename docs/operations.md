# Operator budgets

Read `GET /v1/capabilities` before preparing a recipe.
Its `limits` object contains the active operator budgets. Its `codec_limits` object contains the fixed codec limits.
Operators can raise or lower the defaults.
Generated JSON Schema describes the protocol and numeric representation, independent of deployment settings.

Invalid, zero, overflowing, or inconsistent budgets prevent startup.
The startup codec probe runs inside the configured native resource limits.
Every field in `Limits` has an environment variable in [config](../src/config.rs).
Values ending in `_BYTES` use integer bytes. Values ending in `_MS` use integer milliseconds.

| Setting | Purpose | Default |
| --- | --- | --- |
| `HIRUNE_OUTPUTS` | Maximum outputs in one recipe | 4 |
| `HIRUNE_OUTPUT_BYTES` | Total encoded WebP bytes across outputs | 33554432 |
| `HIRUNE_OUTPUT_DIMENSION` | Maximum requested output side in pixels | 4096 |
| `HIRUNE_WORKING_BYTES` | Working pixel plan: 20 bytes per source canvas pixel | 268435456 |
| `HIRUNE_DECODED_BYTES` | Total decoded RGBA bytes across source frames | 268435456 |
| `HIRUNE_SPOOL_BYTES` | Total RGBA frame file bytes for one request | 268435456 |
| `HIRUNE_CHILD_MEMORY_BYTES` | Linux child virtual address space (`RLIMIT_AS`) | 805306368 |
| `HIRUNE_CHILD_CPU_SECONDS` | Linux child CPU seconds (`RLIMIT_CPU`) | 60 |
| `HIRUNE_CONCURRENCY` | Admitted requests, from upload through delivery | 1 |
| `HIRUNE_DEADLINE_MS` | Absolute wall time through delivery on the server | 60000 |

Set the request envelope budget to at least `source_bytes + recipe_bytes + 1024`.
Actual multipart overhead also counts toward the envelope.
When raising the output count, check recipe, output-byte, and deadline budgets too.
Job and result control file limits grow with recipe bytes and output count.
The writer and reader both check these sizes.

## Fixed limits

| Limit | Value |
| --- | --- |
| WebP side length | 16383 pixels |
| One WebP file | 4294967294 bytes |
| One frame delay | 16777215 ms |
| One cycle duration | 2147483647 ms, limited by decoder timestamps |
| Encoder effort | 0 through 6 |

Animation frame delays must be positive. The RGB input policy and no-enlargement policy also remain fixed.
The output-byte sum can exceed the limit for one RIFF file.
The semaphore API's numeric representation limits concurrency. There is no fixed four-request policy.

## Memory and temporary storage

The worker decodes the source pixels once and stores RGBA frame files in a temporary request directory.
It encodes outputs in sequence.
Set `HIRUNE_TEMP_ROOT` to choose the filesystem for that directory.
The Compose profile mounts `/tmp` as tmpfs. **tmpfs uses container RAM.**
Operators can also use a disk-backed directory. The spool is temporary file storage, not a separate in-process RAM cache.

The working pixel plan does not measure heap memory or resident memory (RSS).
`RLIMIT_AS` limits each child's virtual address space, including libraries and native allocations.
Docker enforces a separate memory limit for the full container.
The API process also holds upload and output buffers.
tmpfs pages, API buffers, concurrent workers, and delivery buffers all count toward the container's cgroup memory.

Allow space for all these resources when sizing the container. Do not multiply only the pixel budget.
A native limit that is too small can prevent startup.
A container limit that is too small can cause the service process to be killed.

## Container settings and larger example

Use these Compose settings to configure the enclosing container:

- `HIRUNE_CONTAINER_MEMORY`: container memory limit.
- `HIRUNE_TMPFS_SIZE`: maximum tmpfs size.
- `HIRUNE_CONTAINER_CPUS`: CPU quota.
- `HIRUNE_CONTAINER_PIDS`: process count limit.

The [larger example](../.envs/.example/large.env) uses eight outputs and a 128 MiB output-byte sum.
It sets 1 GiB pixel and spool budgets, a 2 GiB child address-space limit, and a 4 GiB container limit.
These are example settings, not a measured capacity guarantee.
From the repository root, inspect the configuration, then start the service:

```sh
docker compose --env-file .envs/.example/large.env -f deploy/compose.yml config
docker compose --env-file .envs/.example/large.env -f deploy/compose.yml up -d
```

Read capabilities to confirm the active values. Measure the intended workload before choosing production budgets.
Changing Docker memory does not change application budgets. Changing application budgets does not change Docker memory.
Windows supports functional deadline and process tests. Verify native address-space and CPU enforcement on Linux.

## Verify a profile

Run `just test-container` to check the default profile.
Set `HIRUNE_VERIFY_PROFILE=large` to run the same fixture checks with the larger example.
CI checks both profiles against the exact release candidate.
These checks verify configuration, startup, and fixture behavior. They do not establish capacity at the maximum budgets.
