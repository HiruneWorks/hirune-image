# Hirune and imgproxy service benchmark

Measure one work unit as one source image with all requested output sizes.
Hirune uses one multipart POST. imgproxy uses one GET per output size.
The imgproxy source comes from a fixed in-memory origin on the same internal Docker network.
The benchmark records origin request counts and transferred bytes.

Run from the repository root:

```sh
python benchmarks/imgproxy/run.py --hirune-image ghcr.io/hiruneworks/hirune-image:0.3.0 --output target/benchmarks/baseline.json
```

The tool builds a Python/Pillow client image. It compares existing service images.
imgproxy is pinned to the registry digest for OSS v4.0.20.
The first run downloads the fixed NASA source and prepares fixtures outside measurement.
Later runs reuse the same prepared files and SHA-256 values.
Use `--iterations 1 --warmup 0 --cases photo --outputs 1` for a harness smoke check.

## Fixed conditions

- Both services receive the same source bytes and produce the same exact output dimensions.
- Both use lossy WebP quality 85 and effort 4. These settings do not establish equal perceptual quality.
- Both service containers use a two-CPU quota, 1 GiB memory with swap disabled, a 512 MiB tmpfs, one native processing thread per image, a non-root user, a read-only root filesystem, and no external network. CPU cores are not pinned or dedicated.
- Set work-unit concurrency with `--concurrency`. Hirune admission and imgproxy workers follow it.
- `--fanout serial` requests imgproxy outputs in sequence. `parallel` uses up to N output requests per work unit and records the larger worker count.
- The `accurate` profile enables imgproxy linear colorspace and disables shrink-on-load. The `preview` profile enables linear colorspace but permits shrink-on-load. Neither is imgproxy's complete default configuration.
- `--decode-mode preview` requests Hirune's optional preview mode from 0.4.0. Use `default` for the released 0.3.0 baseline.

HTTP latency includes sending and receiving all outputs for a work unit.
Client validation occurs outside the measured batch. A batch-complete handshake lets the host read service CPU before that validation starts.
The client fully decodes every output with Pillow and checks geometry, alpha regions, orientation markers, animation order markers, every delay, and loops.
Static outputs also receive a coarse visual sanity check against a Pillow sRGB BOX reference. This reference does not rank different resize filters.
The report records generated imgproxy EXIF separately. Hirune's metadata-free contract remains strict.
Hirune's internal output verification and manifest generation remain in its production path.
The benchmark does not add these operations to imgproxy.
CPU uses service cgroup deltas after warmup and before client validation. Origin and client CPU are separate.
Memory is the fresh service cgroup peak, including startup, warmup, tmpfs, and measurement. It is not RSS or a general memory bound.
The client and origin share a separate container with two CPUs and 2 GiB memory.
The resize filters and codec versions can differ. Results are service comparisons, not isolated algorithm comparisons.

## Sources and coverage

The photo source is NASA `iss043e000030`, credited to NASA / the Expedition 43 crew.
Source: https://images-assets.nasa.gov/image/iss043e000030/iss043e000030~orig.jpg
Usage: https://www.nasa.gov/nasa-brand-center/images-and-media/
The tool crops, resizes, re-encodes, and adds EXIF and synthetic-alpha variants.
The benchmark implies no endorsement by NASA.
Generated files and their hashes live under ignored `target/benchmarks/fixtures`.
Reports retain the provenance and generator versions. Successive case/output pairs alternate provider order.
Every started scenario remains in the report, including startup or warmup failures.

Cases include an 11.2 MP photo, a progressive JPEG, odd-sized EXIF-rotated JPEG, a 1537×1025 transparent PNG, and a 638×362 animation with 60 distinct frames.
Outputs use side limits 384, 192, 96, and 48 pixels. All animation outputs require actual resizing.
Use `--workload full` for eight widths spanning thumbnails through large presentations: 160, 320, 640, 960, 1280, 1920, 2560, and 4096, maximum height 4096, quality 90, and effort 4.
The tool raises Hirune's output-count limit to eight for that workload. It records duplicate actual dimensions caused by the no-enlargement rule.
An original-sized result in this profile can prevent reduced JPEG decoding for the whole work unit.
These cases provide bounded regression workloads. They do not represent a production image corpus.

## Recorded comparison

The [2026-11-02 results](../results/2026-11-02/README.md) include accurate and preview modes, serial and parallel imgproxy requests, logical concurrency one and two, and a repeated baseline control.
They retain all raw latency samples, output facts, and resource measurements.
Twenty observations per row provide descriptive measurements, not a tail-latency guarantee.

The benchmark retains first-iteration outputs under each report's `-outputs` directory.
After all measurements finish, compare them with Pillow 13.0.0 in the client image:

```sh
docker run --rm --network none --read-only --mount type=bind,source=REPOSITORY_PATH,target=/work --workdir /work hirune-imgproxy-benchmark-client:local python benchmarks/imgproxy/compare_outputs.py
```

Use the report names in the recorded comparison. The tool checks actual retained bytes and writes `quality-comparison.json` beside the reports.
It compares preview pixels with accurate output. Those statistics do not establish equal perceptual quality between services.
