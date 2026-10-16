"""Compare Hirune and the independent Node/Sharp HTTP baseline with the same Docker resource limits."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import math
from pathlib import Path
import platform
import statistics
import struct
import subprocess
import time
import urllib.error
import urllib.request
import uuid
import zlib
from verify_response import verify

ROOT = Path(__file__).resolve().parents[1]
CLIENT = "python:3.14-slim"
CASES = [f"{name}/{count}" for name in ("opaque", "alpha", "animation", "icc") for count in (1, 4)]

def run(*args, capture=True):
    return subprocess.run(args, check=True, text=True, cwd=ROOT,
                          stdout=subprocess.PIPE if capture else None).stdout

def png(alpha):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    rows = bytearray()
    for y in range(768):
        rows.append(0)
        for x in range(1024):
            rows.extend((x % 256, y % 256, (x ^ y) % 256, (x * 13 + y * 7) % 256 if alpha else 255))
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 1024, 768, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")

def source_for(case):
    name = case.split("/")[0]
    if name in ("opaque", "alpha"):
        return png(name == "alpha")
    return (ROOT / "tests/fixtures" / {"animation": "finite1.gif", "icc": "p3-orientation6-gps.jpg"}[name]).read_bytes()

def recipe_for(count, mode, quality, effort):
    return {"contract_version": 1, "animation_policy": "preserve", "outputs": [
        {"id": f"size_{size}", "resize": {"fit": "inside", "max_width": size, "max_height": size},
         "encoding": {"format": "webp", "mode": mode, "effort": effort, **({"quality": quality} if mode == "lossy" else {})}}
        for size in [512, 256, 128, 64][:count]]}

def fetch(url, data=None, content_type=None):
    request = urllib.request.Request(url, data=data, headers={"Content-Type": content_type} if content_type else {})
    with urllib.request.urlopen(request, timeout=120) as response:
        return response.read()

def validate_workload(manifest, case, recipe, provider):
    name = case.split("/")[0]
    width, height = {"opaque": (1024, 768), "alpha": (1024, 768), "animation": (64, 32), "icc": (40, 84)}[name]
    if name == "animation":
        source = source_for(case)
        assert (width, height) == struct.unpack_from("<HH", source, 6)
    for actual, requested in zip(manifest["outputs"], recipe["outputs"], strict=True):
        mw, mh = requested["resize"]["max_width"], requested["resize"]["max_height"]
        if width <= mw and height <= mh:
            expected = (width, height)
        elif mw * height <= mh * width:
            expected = (mw, max(1, height * mw // width))
        else:
            expected = (max(1, width * mh // height), mh)
        assert (actual["width"], actual["height"]) == expected, "recipe geometry not applied"
        assert actual["duration_ms"] == (1100 if name == "animation" else 0), "cycle duration changed"
        assert actual["loop_count"] == (2 if name == "animation" else 1), "loop count changed"
        assert 1 <= actual["frame_count"] <= (3 if name == "animation" else 1)
        if provider == "hirune":
            assert actual["frame_count"] == (3 if name == "animation" else 1)

def client(args):
    for _ in range(200):
        try:
            fetch(args.base + "/health/ready")
            break
        except (OSError, urllib.error.URLError):
            time.sleep(0.1)
    else:
        raise RuntimeError("service did not become ready")
    info = json.loads(fetch(args.base + "/v1/capabilities"))
    source = source_for(args.case)
    recipe = recipe_for(int(args.case.split("/")[1]), args.mode, args.quality, args.effort)
    raw_recipe = json.dumps(recipe, separators=(",", ":")).encode()
    boundary = "hirune-benchmark"
    body = (f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="fixture"\r\nContent-Type: application/octet-stream\r\n\r\n'.encode() + source +
            f'\r\n--{boundary}\r\nContent-Disposition: form-data; name="recipe"\r\nContent-Type: application/json\r\n\r\n'.encode() + raw_recipe + f"\r\n--{boundary}--\r\n".encode())
    def send(_):
        started = time.perf_counter()
        try:
            payload = fetch(args.base + "/v1/images/process", body, f"multipart/form-data; boundary={boundary}")
            return (time.perf_counter() - started) * 1000, payload, None
        except (OSError, urllib.error.URLError) as error:
            return (time.perf_counter() - started) * 1000, None, str(error)
    def validate(payload):
        manifest = verify(payload, source, raw_recipe, info if args.provider == "hirune" else None)
        validate_workload(manifest, args.case, recipe, args.provider)
        return [{"id": o["id"], "bytes": o["byte_size"], **{k: o[k] for k in ("width", "height", "frame_count", "duration_ms", "loop_count")}} for o in manifest["outputs"]]
    for i in range(args.warmup):
        _, payload, error = send(i)
        if error:
            raise RuntimeError(error)
        validate(payload)
    started = time.perf_counter()
    with ThreadPoolExecutor(max_workers=args.concurrency) as pool:
        results = list(pool.map(send, range(args.iterations)))
    elapsed = time.perf_counter() - started
    latencies, errors, outputs, response_sizes = [], [], [], []
    for latency, payload, error in results:
        if error is None:
            try:
                outputs.append(validate(payload))
                latencies.append(latency)
                response_sizes.append(len(payload))
            except (AssertionError, ValueError, KeyError) as validation_error:
                error = "invalid response: " + repr(validation_error)
        if error:
            errors.append(error)
    latencies.sort()
    print(json.dumps({"provider": args.provider, "case": args.case, "mode": args.mode,
        "iterations": args.iterations, "warmup": args.warmup, "concurrency": args.concurrency,
        "quality": args.quality if args.mode == "lossy" else None, "effort": args.effort,
        "source_sha256": hashlib.sha256(source).hexdigest(), "source_bytes": len(source),
        "successes": len(latencies), "errors": errors, "error_rate": len(errors) / args.iterations,
        "latency_ms": {"p50": statistics.median(latencies) if latencies else None,
                       "p95": latencies[math.ceil(len(latencies) * .95) - 1] if len(latencies) >= 20 else None},
        "successful_requests_per_second": len(latencies) / elapsed, "batch_seconds": elapsed,
        "output_facts": outputs[0] if outputs else [], "response_bytes_mean": statistics.mean(response_sizes) if response_sizes else None,
        "service_info": info}))

def cpu_usage(name):
    fields = run("docker", "exec", name, "cat", "/sys/fs/cgroup/cpu.stat").split()
    return int(dict(zip(fields[::2], fields[1::2]))["usage_usec"])

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--iterations", type=int, default=30)
    parser.add_argument("--warmup", type=int, default=3)
    parser.add_argument("--concurrency", type=int, choices=range(1, 5), default=1)
    parser.add_argument("--mode", choices=("lossy", "lossless"), default="lossy")
    parser.add_argument("--quality", type=int, choices=range(1, 101), default=85)
    parser.add_argument("--effort", type=int, choices=range(7), default=4)
    parser.add_argument("--cpus", default="2")
    parser.add_argument("--memory", default="1g")
    parser.add_argument("--cases", nargs="+", choices=CASES, default=CASES)
    parser.add_argument("--output", type=Path, default=ROOT / "target/benchmarks/services.json")
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--client", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--base", help=argparse.SUPPRESS)
    parser.add_argument("--provider", choices=("hirune", "sharp"), help=argparse.SUPPRESS)
    parser.add_argument("--case", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.iterations < 1 or args.warmup < 0:
        parser.error("iterations must be positive and warmup nonnegative")
    if args.client:
        client(args)
        return
    if not args.skip_build:
        run("docker", "build", "-f", "compose/production/app/Dockerfile", "-t", "hirune-image:benchmark", ".", capture=False)
        run("docker", "build", "-f", "benchmarks/sharp/Dockerfile", "-t", "hirune-sharp:benchmark", ".", capture=False)
    run("docker", "pull", CLIENT, capture=False)
    name = "image-bench-" + uuid.uuid4().hex[:12]
    report = {"schema_version": 1, "host": platform.platform(), "docker": json.loads(run("docker", "info", "--format", "{{json .}}"))["ServerVersion"],
              "hirune_commit": run("git", "rev-parse", "HEAD").strip(), "hirune_dirty": bool(run("git", "status", "--porcelain").strip()),
              "cpus": args.cpus, "memory": args.memory, "tmpfs": "512m", "cpu_scope": "fresh service cgroup cumulative CPU from startup through warmup, measurement and validation", "results": []}
    run("docker", "network", "create", "--internal", name)
    try:
        for case in args.cases:
            for provider in ("hirune", "sharp"):
                image = "hirune-image:benchmark" if provider == "hirune" else "hirune-sharp:benchmark"
                environment = ["HIRUNE_CONCURRENCY=" + str(args.concurrency)] if provider == "hirune" else ["REQUEST_CONCURRENCY=" + str(args.concurrency), "SHARP_CONCURRENCY=1", "VIPS_CONCURRENCY=1"]
                run("docker", "run", "-d", "--name", name, "--network", name, "--read-only", "--tmpfs", "/tmp:rw,noexec,nosuid,size=512m,mode=1777", "--cap-drop", "ALL", "--security-opt", "no-new-privileges:true", "--memory", args.memory, "--memory-swap", args.memory, "--cpus", args.cpus, "--pids-limit", "64", *[item for value in environment for item in ("-e", value)], image)
                try:
                    result = json.loads(run("docker", "run", "--rm", "--network", name, "--read-only", "--mount", f"type=bind,source={ROOT},target=/work,readonly", "-w", "/work", "-e", "PYTHONDONTWRITEBYTECODE=1", CLIENT, "python", "scripts/benchmark_services.py", "--client", "--base", f"http://{name}:8080", "--provider", provider, "--case", case, "--iterations", str(args.iterations), "--warmup", str(args.warmup), "--concurrency", str(args.concurrency), "--quality", str(args.quality), "--effort", str(args.effort), "--mode", args.mode))
                    result["cgroup_cpu_seconds"] = cpu_usage(name) / 1e6
                    result["cgroup_memory_peak_bytes"] = int(run("docker", "exec", name, "cat", "/sys/fs/cgroup/memory.peak"))
                    result["image_id"] = run("docker", "image", "inspect", image, "--format", "{{.Id}}").strip()
                    report["results"].append(result)
                    print(f"{provider} {case}: p50={result['latency_ms']['p50']}ms, successes={result['successes']}/{args.iterations}", flush=True)
                finally:
                    subprocess.run(["docker", "rm", "-f", name], stdout=subprocess.DEVNULL, check=False)
    finally:
        subprocess.run(["docker", "network", "rm", name], stdout=subprocess.DEVNULL, check=False)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if any(result["errors"] for result in report["results"]):
        raise SystemExit("benchmark has failed responses; inspect " + str(args.output))

if __name__ == "__main__":
    main()
