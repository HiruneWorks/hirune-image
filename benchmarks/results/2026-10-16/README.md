# Local benchmark evidence: 2026-10-16

Standalone Sharp 0.35.5 / libvips 8.18.7 / WebP 1.6.0 / Node 24.22.0 versus Hirune 0.1.0 / WebP 1.5.0. No Smol dependency. See [method](../../../docs/benchmarks.md) and [environment](environment.json).

Each row uses 20 measured requests after three warmups, with two CPUs and 1GiB memory per service. All 480 measured requests and 72 warmups passed source/recipe/output binding, requested geometry and animation cycle/loop checks. Tables show HTTP p50/p95 in milliseconds and fresh-cgroup peak memory in MiB; CPU, bytes and throughput remain in the raw JSON. Synthetic PNG patterns and small handoff animation/ICC fixtures are not a representative production corpus. Resize filters and generated pixels differ, so neither perceptual quality nor compression efficiency is held equal.

## Lossy, quality 85 / effort 4 / concurrency 1

| Case | Hirune p50 / p95 ms | Sharp p50 / p95 ms | Hirune / Sharp peak MiB |
| --- | --- | --- | --- |
| opaque/1 | 55.67 / 80.86 | 44.74 / 53.59 | 12.27 / 83.40 |
| opaque/4 | 68.02 / 75.05 | 67.47 / 72.90 | 13.68 / 73.31 |
| alpha/1 | 66.38 / 70.04 | 72.38 / 75.74 | 19.42 / 91.78 |
| alpha/4 | 93.80 / 97.85 | 116.04 / 118.94 | 18.16 / 81.05 |
| animation/1 | 5.43 / 8.82 | 7.04 / 7.73 | 4.40 / 34.07 |
| animation/4 | 8.74 / 10.61 | 19.51 / 26.48 | 3.90 / 38.23 |
| icc/1 | 5.16 / 7.92 | 10.56 / 13.45 | 3.64 / 32.72 |
| icc/4 | 6.88 / 9.81 | 20.19 / 23.01 | 4.14 / 35.86 |

[Raw sharp-lossy report](sharp-lossy.json).

## Lossless, effort 4 / concurrency 1

| Case | Hirune p50 / p95 ms | Sharp p50 / p95 ms | Hirune / Sharp peak MiB |
| --- | --- | --- | --- |
| opaque/4 | 127.29 / 174.26 | 147.72 / 185.74 | 22.88 / 86.62 |
| alpha/4 | 136.21 / 149.48 | 219.04 / 225.97 | 24.04 / 88.25 |
| animation/4 | 6.52 / 7.69 | 15.34 / 18.61 | 3.90 / 37.87 |

[Raw sharp-lossless report](sharp-lossless.json).

## Lossy, quality 85 / effort 4 / concurrency 2

| Case | Hirune p50 / p95 ms | Sharp p50 / p95 ms | Hirune / Sharp peak MiB |
| --- | --- | --- | --- |
| opaque/4 | 66.51 / 76.34 | 76.72 / 84.47 | 22.66 / 103.80 |

[Raw sharp-concurrency2 report](sharp-concurrency2.json).

Sharp is faster on the single-output opaque PNG (44.74 ms versus 55.67 ms), and the four-output opaque lossy case is close. Hirune is faster on the tested alpha, animation, ICC and lossless cases, with lower cgroup peaks here. These results do not establish a universal winner.

## Criterion components

The [Windows Criterion estimates](criterion-windows.json) cover decode, orientation, linear area resize and lossy/lossless encode. This short development run validates the benchmark configuration; use a quiet, fixed host and longer sampling for a regression baseline. It is separate from the Linux Docker comparison.

## Functional verification

Local Windows workspace checks passed (31 tests, followed by the new fixture-integrity test: 32). Linux `cargo xtask check` passed 33 tests including actual AS/CPU exhaustion, PID disappearance and next-request recovery. Both production-restriction Docker profiles passed eight fixture HTTP cases each. The independent Sharp image passed its three component tests before these measurements. Remote CI and deployment are separate evidence.
