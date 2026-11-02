# Local imgproxy comparison â€” 2026-11-02

These results describe fixed developer-host workloads. They do not establish production capacity or a universal ranking.

Multi-output and animation workloads show substantial reductions in these runs. The 41 retained accurate baseline/candidate WebP samples are byte-identical across the one, four, and eight-output profiles.
JPEG preview trades pixels for less work and lower measured memory. It remains optional.
imgproxy remained faster for single JPEG outputs. Parallel URL requests also changed some latency rankings.
All 88 scenario rows passed: 1,760 measured work units, 5,480 output WebPs, and 3,420 processing HTTP requests. These counts exclude warmup and health requests.

## Environment and method

- Host: Windows 11, AMD Ryzen 7 8845HS, 8 physical / 16 logical CPUs. Docker Desktop Linux engine 28.3.3 had 4 CPUs and about 16 GiB available.
- Each service had a two-CPU quota, 1 GiB memory with swap disabled, 512 MiB tmpfs, and 64 PID limit. CPU cores were not pinned or dedicated.
- Each scenario used a fresh non-root, read-only service container. The separate client/origin container had a two-CPU quota and 2 GiB memory.
- Each row contains 20 measured work units after two warmup units. One unit finishes all requested sizes of one source image.
- Baseline: published Hirune 0.3.0. Candidate: 0.4.0 code at `ac26bdd892ff25dfbcedafa8fcbcb869b9ea4ffc`. imgproxy: OSS 4.0.20.
- Image digests, source hashes, generator versions, settings, attempted request counts, and output facts remain in the raw JSON files.
- The accurate imgproxy profile uses linear colorspace and disables shrink-on-load. The preview profile uses linear colorspace and permits shrink-on-load. Neither is its complete default configuration.
- Hirune sends source bytes once per work unit. imgproxy fetches the in-memory origin once per output URL. Its operation cache is disabled by its v4.0.20 initialization.
- Client validation runs after the timed batch and CPU handshake. Every output is fully decoded and checked for dimensions, alpha, orientation, frame order, delays, and loops.

The three JPEG cases are variants of one NASA photograph, not three independent photographs. Alpha PNG and the 60-frame animation are synthetic.
See the [harness method and source credit](../../imgproxy/README.md). Reports preserve the exact fixture provenance.

## Accurate mode: one and four outputs

All latency cells are p50 / p95 milliseconds for the complete work unit. Lower is better.
imgproxy requests are sequential in this table. Quality is 85 and effort is 4 for both services.
The second baseline run followed the candidate measurements. It checks time drift without replacing independent repeated experiments.

| Input | Outputs | 0.3 initial | 0.3 repeat | 0.4 accurate | imgproxy accurate | p50 reduction vs repeat |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| photo | 1 | 189.2 / 229.8 | 172.5 / 177.7 | 165.5 / 179.5 | 156.3 / 165.5 | 4.1% |
| photo | 4 | 341.7 / 385.0 | 278.8 / 286.7 | 218.1 / 231.2 | 416.5 / 443.3 | 21.8% |
| progressive | 1 | 424.4 / 478.5 | 348.4 / 366.4 | 369.6 / 439.7 | 277.1 / 335.9 | -6.1% |
| progressive | 4 | 493.6 / 548.4 | 458.6 / 472.8 | 423.5 / 504.4 | 904.8 / 1097.2 | 7.6% |
| rotated | 1 | 263.3 / 296.5 | 228.5 / 234.2 | 237.7 / 294.3 | 159.7 / 200.3 | -4.0% |
| rotated | 4 | 359.4 / 375.6 | 337.1 / 351.1 | 269.8 / 279.6 | 420.1 / 485.6 | 20.0% |
| alpha | 1 | 74.5 / 81.3 | 68.6 / 70.6 | 67.1 / 71.5 | 110.3 / 132.2 | 2.1% |
| alpha | 4 | 92.7 / 95.7 | 90.8 / 94.0 | 77.3 / 81.5 | 339.2 / 440.3 | 14.9% |
| animation | 1 | 765.4 / 781.8 | 755.2 / 764.6 | 382.2 / 404.1 | 661.6 / 673.7 | 49.4% |
| animation | 4 | 1151.7 / 1203.0 | 1121.4 / 1134.0 | 561.5 / 592.4 | 1725.9 / 1792.4 | 49.9% |

Single-output progressive and rotated JPEG rows were slower than the repeated baseline. Timing drift and decode cost limit conclusions about small differences.

Resource cells are service CPU milliseconds per successful work unit / fresh cgroup peak MiB.
Peak includes startup, warmup, tmpfs, measurement, and probes. It is not RSS, per-request memory, or a general upper bound.
Origin and client resource use are excluded from service CPU and memory.

| Input | Outputs | 0.3 repeat CPU / peak | 0.4 accurate CPU / peak | imgproxy CPU / peak |
| --- | ---: | ---: | ---: | ---: |
| photo | 1 | 173.4 / 104.2 | 166.4 / 105.2 | 166.4 / 117.6 |
| photo | 4 | 279.3 / 100.4 | 220.1 / 104.3 | 449.0 / 107.2 |
| progressive | 1 | 351.6 / 130.1 | 377.0 / 128.9 | 299.9 / 269.9 |
| progressive | 4 | 459.7 / 128.2 | 430.0 / 124.7 | 973.6 / 272.3 |
| rotated | 1 | 229.1 / 112.1 | 250.0 / 114.8 | 178.0 / 99.9 |
| rotated | 4 | 337.5 / 111.0 | 268.9 / 112.2 | 453.7 / 103.0 |
| alpha | 1 | 68.6 / 39.8 | 67.5 / 35.0 | 121.0 / 85.3 |
| alpha | 4 | 91.6 / 36.4 | 77.2 / 37.2 | 376.2 / 92.1 |
| animation | 1 | 755.7 / 6.4 | 386.0 / 6.6 | 696.2 / 58.4 |
| animation | 4 | 1122.5 / 7.4 | 566.7 / 7.3 | 1797.9 / 56.5 |

## Optional JPEG preview

Both providers permit reduced decoding here. Hirune uses explicit `decode_mode=preview`.
Latency cells are p50 / p95 ms. Resources are CPU ms per unit / cgroup peak MiB.

| Input | Outputs | Hirune preview | imgproxy preview | Hirune CPU / peak | imgproxy CPU / peak |
| --- | ---: | ---: | ---: | ---: | ---: |
| photo | 1 | 108.6 / 116.8 | 66.1 / 101.0 | 119.9 / 32.1 | 75.3 / 31.6 |
| photo | 4 | 107.6 / 110.2 | 166.5 / 176.8 | 117.6 / 30.0 | 181.3 / 40.6 |
| progressive | 1 | 240.7 / 263.8 | 157.4 / 166.1 | 257.5 / 89.6 | 171.0 / 160.7 |
| progressive | 4 | 247.5 / 253.1 | 575.6 / 586.5 | 259.0 / 90.5 | 612.6 / 183.4 |
| rotated | 1 | 107.0 / 110.1 | 52.7 / 58.7 | 120.5 / 36.0 | 57.4 / 31.3 |
| rotated | 4 | 118.0 / 123.5 | 172.6 / 176.1 | 131.7 / 29.9 | 186.6 / 37.8 |

## Eight-output full profile

Widths are 160, 320, 640, 960, 1280, 1920, 2560, and 4096 with maximum height 4096, quality 90, and effort 4.
Requests use accurate decoding and sequential imgproxy URLs. A full-size result prevents shared reduced JPEG decoding.
No enlargement produces eight unique photo sizes but only three unique animation sizes. All eight requested parts are still produced.

| Input | 0.3 p50 / p95 ms | 0.4 p50 / p95 ms | imgproxy p50 / p95 ms |
| --- | ---: | ---: | ---: |
| photo | 3395.1 / 4667.1 | 2791.9 / 2955.8 | 3414.1 / 3583.8 |
| animation | 3446.6 / 4190.0 | 2650.5 / 2817.9 | 4691.1 / 6483.8 |

## Parallel imgproxy fanout

Each work unit requests four output URLs concurrently from imgproxy. Hirune retains one batch POST.
At logical concurrency C, imgproxy has CÃ—4 workers and Hirune has C admission permits. Both keep the same two-CPU and 1 GiB quotas.
This is an explicit worker configuration, not a claim that it is optimal for every input.

| Mode | Logical concurrency | Input | Hirune p50 / p95 ms | imgproxy p50 / p95 ms | Hirune units/s | imgproxy units/s |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| accurate | 1 | photo | 217.8 / 229.7 | 199.0 / 233.3 | 4.55 | 4.87 |
| accurate | 1 | animation | 547.6 / 571.3 | 1544.1 / 1699.4 | 1.81 | 0.64 |
| accurate | 2 | photo | 216.9 / 254.5 | 444.3 / 583.9 | 8.90 | 4.35 |
| accurate | 2 | animation | 593.6 / 732.4 | 3728.6 / 4536.9 | 3.25 | 0.54 |
| preview | 1 | photo | 105.3 / 108.2 | 89.4 / 99.2 | 9.50 | 11.19 |
| preview | 1 | progressive | 240.3 / 246.0 | 303.0 / 340.1 | 4.15 | 3.28 |
| preview | 1 | rotated | 114.0 / 117.0 | 94.0 / 103.7 | 8.75 | 10.67 |

## Output fidelity and measurement limits

The retained accurate samples match byte-for-byte for 41 case/size entries. The repeated 0.3 baseline also matches its original 25 thumbnail samples.
This is evidence for the tested cases. Separate scalar-reference tests cover randomized odd geometry, alpha boundaries, and transfer rounding.
Across 15 retained JPEG preview samples, mean absolute RGB code differences against accurate output range from 2.976 to 4.790. RGB PSNR ranges from 31.56 to 36.23 dB.
These compare decoded opaque sRGB pixels after lossy WebP q85/effort4. They do not rank perceptual quality or establish a fidelity threshold.
See [quality-comparison.json](quality-comparison.json) for every sample and its hash.

- Area and Lanczos3 filters differ. Equal quality and effort values do not mean equal visual quality or equal output size.
- Hirune performs full output verification and manifest generation in its production path. The harness does not add that work to imgproxy.
- imgproxy generated EXIF is recorded separately. Hirune outputs retain the metadata-free contract.
- Twenty observations form one batch per row. The reported p95 is the second-largest sample, not a tail-latency guarantee or confidence interval.
- CPU frequency, thermal state, Docker scheduling, and shared-host activity can vary. Small differences should not be treated as universal improvements.
- Functional tests cover more formats than this performance corpus. These numbers do not establish HEIC, AVIF, JPEG XL, SVG, or production-workload performance.
- Astra reviewed the harness, optimization code, and completed thumbnail/preview data. The review found no measurement-invalidating issue and required these interpretation limits.

## Raw reports

- [baseline-accurate.json](baseline-accurate.json)
- [baseline-confirmation.json](baseline-confirmation.json)
- [reference-preview.json](reference-preview.json)
- [candidate-accurate.json](candidate-accurate.json)
- [candidate-preview.json](candidate-preview.json)
- [baseline-full.json](baseline-full.json)
- [candidate-full.json](candidate-full.json)
- [candidate-parallel.json](candidate-parallel.json)
- [candidate-parallel-c2.json](candidate-parallel-c2.json)
- [candidate-preview-parallel.json](candidate-preview-parallel.json)
