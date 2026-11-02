# 2026-11-02 — imgproxy comparison results

## Changes

Record the [complete local comparison](../benchmarks/results/2026-11-02/README.md) and ten raw benchmark reports.
Compare published Hirune 0.3.0, the 0.4.0 candidate at `ac26bdd892ff25dfbcedafa8fcbcb869b9ea4ffc`, and imgproxy OSS 4.0.20.
Keep the original baseline and a second baseline control. Preserve both sets of timing observations.
Add an output-comparison tool that checks retained bytes and measures preview differences against accurate output.

Each service used a two-CPU quota, 1 GiB memory with swap disabled, a 512 MiB tmpfs, and 64 PID limit.
The Docker Desktop Linux VM had four CPUs and about 16 GiB memory on an AMD Ryzen 7 8845HS Windows host.
The client and in-memory origin ran separately. Each row had two warmup units and 20 measured units.
One work unit generated every requested size for one source.
Profiles covered accurate and preview decoding, one/four/eight outputs, serial/parallel imgproxy URLs, and logical concurrency one/two.

## Decisions and constraints

Keep the service, documents, and workload names independent of callers. Record that requirement in `AGENTS.md`.
Watermarking remains outside the requirements.
Default decoding remains accurate. JPEG preview is explicit and can change pixels.
Same quality and effort do not establish equal perceptual quality across different resize filters and codecs.
The JPEG cases are variants of one NASA photograph. The alpha and animation inputs are synthetic.

Measured p50 observations include:

- Four-output photo: repeated 0.3 baseline 278.8 ms, candidate accurate 218.1 ms.
- Four-output animation: repeated baseline 1121.4 ms, candidate accurate 561.5 ms.
- Eight-output photo: baseline 3395.1 ms, candidate 2791.9 ms, imgproxy sequential 3414.1 ms.
- With parallel URLs at logical concurrency one, imgproxy photo took 199.0 ms and Hirune accurate took 217.8 ms.
- At logical concurrency two, photo throughput was 8.90 units/s for Hirune and 4.35 units/s for imgproxy. That profile used eight imgproxy workers.
- Preview with parallel URLs: photo took 105.3 ms for Hirune and 89.4 ms for imgproxy; progressive JPEG took 240.3 ms and 303.0 ms respectively.

Single-output progressive and rotated JPEG observations were slower than the repeated baseline.
CPU frequency, thermal state, and shared-host scheduling were not controlled. Small differences do not establish a universal improvement.
Twenty observations form one batch per row. p95 is descriptive, not a production guarantee.
Cgroup peak includes startup, warmup, tmpfs, and probes. It is not RSS or a per-request memory bound.
Service CPU excludes the separate origin and client.

Astra reviewed the harness, implementation, and completed initial thumbnail/preview reports.
It found no measurement-invalidating issue. Its review required explicit limits for sample size, filters, source coverage, fanout, and memory interpretation.
The report includes later parallel, repeated-baseline, and full-profile observations separately.

## Validation

- All 88 scenario rows passed: 1760 measured work units, 5480 verified WebP outputs, and 3420 processing HTTP requests.
- Those counts exclude warmup and health requests. All measured attempted processing requests matched their planned counts.
- Every output passed full Pillow decoding, geometry, alpha, orientation, animation frame-order, timing, and loop checks.
- The comparator checked actual retained bytes: all 41 accurate baseline/candidate case-size entries matched exactly.
- The repeated baseline matched its original 25 thumbnail samples.
- Fifteen JPEG preview samples differed from accurate output by mean absolute RGB code values from 2.976 to 4.790. RGB PSNR ranged from 31.56 to 36.23 dB.
- Those statistics compare decoded opaque sRGB pixels after lossy WebP q85/effort4. They do not rank perceptual quality.
- `cargo xtask check` passed on Windows with 59 tests. Initial local attempts found new Clippy chunk-API warnings and an unsupported decoder in a new test helper. Both were fixed before the successful complete run.
- Local Linux runtime checks passed in default and large profiles: 22 HTTP processing cases per profile, non-root UID 65532, read-only root, cleanup, and resource settings.
- [PR CI run 41388357941](https://github.com/HiruneWorks/hirune-image/actions/runs/41388357941) passed both `verify` and `container`. Linux `verify` passed 60 tests, including resource exhaustion and recovery. Publication was skipped for the branch.
- Python syntax, output-comparator execution, report consistency assertions, generic-name scans, and `git diff --check` passed.
- The harness removed its scenario containers and networks. Source downloads and sampled outputs remain ignored under `target/`.

## Remaining work

Complete final report review and head CI. Integrate the PR and publish version 0.4.0 through tag CI.
No production deployment or caller integration ran for this comparison milestone.
