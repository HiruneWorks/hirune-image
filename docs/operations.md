# Operator budgets

`GET /v1/capabilities` returns the active operator `limits` and separate immutable `codec_limits`. Defaults are starting budgets, and may be raised or lowered. Generated JSON Schema describes the protocol and codec representation; it is independent of deployment settings. Callers should discover active budgets before preparing a recipe. Invalid, zero, overflowing or inconsistent budgets fail startup. The startup probe runs inside the configured native fence.

Every field in `Limits` has a matching environment variable in [config](../src/config.rs). Values ending in `_BYTES` are integer bytes; `_MS` are integer milliseconds. Examples:

| Setting | Purpose | Default |
| --- | --- | --- |
| `HIRUNE_OUTPUTS` | Maximum variants in one recipe | 4 |
| `HIRUNE_OUTPUT_BYTES` | Sum of encoded WebP bytes across variants | 33554432 |
| `HIRUNE_OUTPUT_DIMENSION` | Maximum requested output side | 4096 |
| `HIRUNE_WORKING_BYTES` | Conservative live-pixel plan: 20 bytes per source canvas pixel | 268435456 |
| `HIRUNE_DECODED_BYTES` | Cumulative decoded RGBA across all source frames | 268435456 |
| `HIRUNE_SPOOL_BYTES` | Cumulative RGBA frame files per request | 268435456 |
| `HIRUNE_CHILD_MEMORY_BYTES` | Linux child virtual address space (`RLIMIT_AS`) | 805306368 |
| `HIRUNE_CHILD_CPU_SECONDS` | Linux child consumed CPU seconds (`RLIMIT_CPU`) | 60 |
| `HIRUNE_CONCURRENCY` | Admitted requests including upload, processing and delivery | 1 |
| `HIRUNE_DEADLINE_MS` | Absolute wall time through server-side delivery | 60000 |

The request envelope must fit `source_bytes + recipe_bytes + 1024`; actual multipart overhead also counts. Raising the output count may require raising recipe, output, and deadline budgets. Job/result control files scale with configured recipe bytes and actual output count, with checked sizes on both ends.

Fixed limits: WebP side 16383, individual WebP file 4294967294 bytes, frame delay 16777215 ms, cycle duration 2147483647 ms (decoder timestamp range), positive animation delays, RGB input policy, effort 0–6, no enlargement. Aggregate output bytes may exceed the individual RIFF file cap. Concurrency is bounded by the semaphore API representation, not a four-request policy.

## Memory and temporary storage

The worker decodes the source pixels once and stores RGBA frame files in a per-request temporary directory. Variants are encoded sequentially. `HIRUNE_TEMP_ROOT` chooses that filesystem. The Compose profile mounts `/tmp` as tmpfs, which **uses container RAM**. A disk-backed directory is also possible. This is not an extra in-process RAM cache.

Pixel planning is not a heap or RSS measurement. `RLIMIT_AS` constrains each child's virtual address space, including libraries and native allocations; it is not the Docker memory limit. The parent also holds upload and output buffers. tmpfs pages, parent buffers, concurrent workers and delivery buffers all count toward the container's cgroup memory. Container memory must leave room for all of them; multiplying the pixel budget alone is insufficient. Overly small native limits can prevent startup; overly small cgroup limits can kill the service.

Compose settings `HIRUNE_CONTAINER_MEMORY`, `HIRUNE_TMPFS_SIZE`, `HIRUNE_CONTAINER_CPUS`, and `HIRUNE_CONTAINER_PIDS` configure the enclosing container. The [larger example](../.envs/.example/large.env) sets eight outputs, 128MiB aggregate output, 1GiB pixel/spool budgets, a 2GiB child AS fence, and a 4GiB container. It is an example, not a measured capacity guarantee:

```sh
docker compose --env-file .envs/.example/large.env -f deploy/compose.yml config
docker compose --env-file .envs/.example/large.env -f deploy/compose.yml up -d
```

Inspect effective values through capabilities and measure the intended workload. Raising Docker memory does not raise application budgets, and raising application budgets does not raise Docker memory. Windows has functional deadline/process tests; native AS/CPU enforcement requires Linux.

`just test-container` verifies the default profile. Setting `HIRUNE_VERIFY_PROFILE=large` selects the larger example for the same fixture checks; CI verifies both profiles against the exact release candidate. This confirms configuration/startup/functional behavior, not capacity at the maximum budgets.
