# Benchmarks

Use Criterion to measure Rust pipeline components.
Use the [Node/Sharp baseline](../benchmarks/sharp/) to compare complete HTTP requests in Docker.
The tools do not depend on Smol. CI has no pass/fail speed threshold.

## Rust components

Run `just bench` or `cargo bench --locked --bench image_pipeline` from the repository root.
The release-mode benchmarks measure PNG decoding, orientation, area resize in linear light, and lossy or lossless WebP encoding.
Input pixels and PNG bytes are prepared before timing starts.

| Operation | Included in timing | Excluded from timing |
| --- | --- | --- |
| Decode, resize, encode | Output allocation and drop | Input preparation |
| Orientation | Orientation of the prepared input | Input cloning and output drop, through `iter_batched` |

Criterion writes reports to `target/criterion`.
Compare baselines on the same machine with the same compiler.
The Criterion [timing-loop guide](https://bheisler.github.io/criterion.rs/book/user_guide/timing_loops.html) explains these measurement boundaries.

The manual `Benchmarks` workflow runs Criterion and saves reports.
Normal CI compiles the benchmarks and checks them with clippy. It does not use a speed threshold.
The [2026-10-16 local results](../benchmarks/results/2026-10-16/README.md) record measurements and their limits.

## Run the Sharp comparison

Docker is required. Run this command from the repository root:

```sh
python scripts/benchmark_services.py --iterations 30 --warmup 3 --output target/benchmarks/services.json
```

Alternatively, run `just bench-services --iterations 30` to select the platform's Python command.
For a short tool check, use `--iterations 3 --warmup 1`.
Use `--skip-build` to reuse the named local benchmark images. The report records their image IDs.

By default, the tool builds Hirune's production Dockerfile and the Sharp benchmark Dockerfile.
The Sharp image uses its committed npm lockfile. Only that image needs Node and npm.
The production service image has no JavaScript runtime or Sharp dependency.

The default comparison uses lossy WebP with quality 85 and effort 4.
Use `--mode lossless` to compare lossless output.

## Workload and resource settings

Each service gets a new, read-only Linux container for each scenario.
Both containers use the same CPU quota, memory limit, PID limit, and tmpfs limit.
They communicate with a separate Python client over an internal Docker network.

The inputs are opaque PNG patterns, transparent PNG patterns, an RGB ICC JPEG, and an animated GIF.
Each input produces either one or four output sizes.
Both services use the same input bytes, quality, effort, dimensions, no-enlargement rule, and admitted request count.
Both decode the source pixels once and encode outputs in sequence.
Sharp uses one native thread and disables its operation cache.

The client reads each full HTTP response before stopping its request timer.
After the timed batch, it checks source, recipe, and output hashes.
It also checks multipart structure, WebP facts, requested dimensions, cycle duration, and loop count.
Expected dimensions are calculated independently from the service.

## Read the report

The JSON report contains these measurements and identifiers:

- Successful requests per second and error rate.
- Latency p50, and p95 when at least 20 samples succeed.
- Encoded output facts and byte counts.
- Service and native library versions, Git revisions, working-tree status, and image IDs.
- CPU usage and peak memory for the service's container resource group, or cgroup.

Request latency covers client send through receipt of the full response body.
Batch throughput includes client scheduling and transport overhead. It excludes the validation after the batch.
CPU usage accumulates from container startup through warmup, measured requests, and validation.
It is not a CPU measurement for one request.
`memory.peak` includes the scenario's tmpfs use. It is not resident memory (RSS).
The client's CPU and memory are outside the service cgroup.
The client opens fresh urllib connections, so connection setup is part of the measurement.

## Comparison limits

The Sharp baseline checks source headers and decodes the source pixels once into oriented sRGB RGBA.
For each output, it resizes, encodes, and fully decodes the result for verification.
It then produces SHA-256 bindings, a manifest, and a multipart response.
Sharp holds decoded pixels in process RAM. Hirune uses a supervised child and temporary RGBA files with byte limits.

The baseline is a performance tool for trusted fixtures.
It does not implement the service's full policy for hostile input, admission cancellation, or child process isolation.

The resize filters differ: Sharp uses Lanczos3; Hirune uses an area filter in linear light with premultiplied alpha.
The services also use different color engines, WebP versions, and animation optimizations.
Sharp can combine frames. The report records actual frame counts, cycle durations, and loops to show this difference.
Equal quality settings do not establish equal perceptual quality.
A small synthetic run does not establish production superiority.
The comparison supports lossy and lossless output. It runs no video workload.
Sharp's [raw frame stack](https://sharp.pixelplumbing.com/api-constructor/) and [WebP/ICC options](https://sharp.pixelplumbing.com/api-output/) define its native pipeline.
