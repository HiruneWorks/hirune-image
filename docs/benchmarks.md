# Benchmarks

Use two layers: Criterion for Rust pipeline changes and a Docker HTTP comparison against the self-contained [Node/Sharp baseline](../benchmarks/sharp/). Neither is a pass/fail performance threshold in CI. The benchmark has no dependency on the Smol checkout or image.

## Rust components

`just bench` (or `cargo bench --locked --bench image_pipeline`) measures PNG decode, orientation, linear area resize, and lossy/lossless WebP encode in release mode. Input pixels/PNG preparation happen before timing. Decode/resize/encode include allocation and output drop; orientation uses `iter_batched` to exclude input cloning and output drop. Criterion reports go to `target/criterion`; compare baselines on the same machine and compiler. [`Timing loops`](https://bheisler.github.io/criterion.rs/book/user_guide/timing_loops.html) explains the measurement boundaries.

The manual `Benchmarks` workflow runs Criterion and saves reports. Normal CI compiles/clippy-checks benchmarks without a flaky speed gate.

See the [2026-10-16 local results](../benchmarks/results/2026-10-16/README.md) for measurements and their limits.

## Sharp service comparison

```sh
python scripts/benchmark_services.py --iterations 30 --warmup 3 --output target/benchmarks/services.json
```

`just bench-services --iterations 30` selects the platform's Python command. Use `--iterations 3 --warmup 1` for a harness smoke test. `--skip-build` reuses the explicitly named local benchmark images; image IDs are recorded. By default the harness builds Hirune's production Dockerfile and the standalone Sharp Dockerfile using its committed npm lockfile. Node/npm are only needed in the benchmark image. Normal service builds contain no JavaScript runtime or Sharp dependency. `--mode lossless` compares lossless operations; default mode is lossy with quality 85 and effort 4.

Each scenario/provider gets a fresh, isolated, read-only Linux container with the same CPU/memory/PID/tmpfs limits and an internal Docker network. A separate Python client reads full HTTP responses, then validates source/recipe/output hashes, multipart structure, WebP facts, independently calculated requested geometry and expected cycle/loop outside the timed batch. PNG opaque/alpha patterns, RGB ICC JPEG, and animated GIF are tested with one/four output sizes. Inputs, quality, effort, dimensions, no-upscale and admitted request concurrency match. Sharp uses one native thread and disables its operation cache; both services decode source pixels once and encode variants sequentially.

The JSON report records successful requests/s, error rate, p50, p95 only with at least 20 successful samples, encoded output facts/bytes, service/native versions, Git revisions, dirty status, image IDs, and cgroup CPU/memory. Latency covers client send through full body read; batch throughput includes client scheduling/transport overhead and excludes post-batch validation. CPU is cumulative usage in the fresh service cgroup from startup through warmup, measurement and validation, not per-request CPU. memory.peak covers the scenario including tmpfs, not RSS. Client CPU/memory are outside the service cgroup. The client uses fresh urllib connections; this is a declared transport cost, not a codec-only benchmark.

The Sharp baseline performs header inspection, one full source pixel decode to oriented sRGB RGBA, sequential resize/encode, full output decode verification, SHA-256, manifest and multipart. It keeps decoded pixels in its process RAM; Hirune uses a supervised child and bounded RGBA files. The baseline is a trusted-fixture performance tool, not a production replacement: it does not implement Hirune's complete adversarial-input policy, admission cancellation or OS child isolation.

The services have different resize filters (Sharp Lanczos3 versus Hirune linear premultiplied area), color engines, WebP versions and animation optimization policies. Sharp can coalesce frames; the report retains actual frame/time/loop facts so these differences remain visible. Equal quality numbers do not establish equal perceptual quality, and a small synthetic run does not establish production superiority. Both lossless and lossy modes are available. No video workload runs. Sharp's [raw frame stack](https://sharp.pixelplumbing.com/api-constructor/) and [WebP/ICC options](https://sharp.pixelplumbing.com/api-output/) define the baseline's native pipeline.
