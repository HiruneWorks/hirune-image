# Local benchmark evidence: 2026-10-16

This run compared Sharp 0.35.5, libvips 8.18.7, WebP 1.6.0, and Node 24.22.0 with Hirune 0.1.0 and WebP 1.5.0.
The benchmark had no Smol dependency. See the [method](../../../docs/benchmarks.md) and [environment](environment.json).

Each row uses 20 measured requests after three warmups. Each service had two CPUs and 1 GiB of container memory.
All 480 measured requests and 72 warmups passed binding, requested-dimension, cycle-duration, and loop-count checks.
Binding checks covered the source, recipe, and outputs.
The tables show HTTP p50 and p95 in milliseconds, plus peak memory in a new cgroup for each scenario.
Memory values use MiB. Raw JSON records CPU usage, byte counts, and throughput.

The inputs were synthetic PNG patterns and small animation and ICC fixtures from the handoff.
They do not represent a production image corpus.
The resize filters produce different pixels. The run does not hold perceptual quality or compression efficiency equal.

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

Sharp was faster on the opaque PNG with one output: 44.74 ms versus 55.67 ms.
The opaque lossy case with four outputs was close.
Hirune was faster on these alpha, animation, ICC, and lossless fixtures. Its cgroup memory peaks were lower in this run.
These results do not establish a universal winner.

## Criterion components

The [Windows Criterion estimates](criterion-windows.json) cover decoding, orientation, area resize in linear light, and lossy or lossless encoding.
This short development run checked that the benchmark configuration works.
Use a quiet, fixed host and longer sampling to establish a regression baseline.
These measurements are separate from the Linux Docker comparison.

## Functional verification

Local Windows workspace checks passed 31 tests. The later fixture-integrity test also passed, for a total of 32.
Linux `cargo xtask check` passed 33 tests.
These included address-space and CPU exhaustion, child PID disappearance, and recovery for the next request.
The default and larger Docker profiles each passed eight fixture HTTP cases under production restrictions.
The independent Sharp image passed its three component tests before measurement.
Remote CI and deployment remain separate evidence.
