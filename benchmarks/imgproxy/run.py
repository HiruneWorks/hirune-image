"""Compare existing Hirune and imgproxy images under identical container limits."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import queue
import subprocess
import sys
import threading
import time
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[2]
CLIENT = "hirune-imgproxy-benchmark-client:local"
IMGPROXY = "ghcr.io/imgproxy/imgproxy@sha256:f5db62d528ab08e7b12f14e759fd7d62c09aeabf2e629d0e2a3963da692d0130"
PHOTO = "https://images-assets.nasa.gov/image/iss043e000030/iss043e000030~orig.jpg"


def run(*args):
    return subprocess.run(args, cwd=ROOT, check=True, text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE).stdout


def ensure_image(image):
    if subprocess.run(["docker", "image", "inspect", image], stdout=subprocess.DEVNULL,
                      stderr=subprocess.DEVNULL).returncode:
        print(run("docker", "pull", image), flush=True)
    return run("docker", "image", "inspect", image, "--format", "{{.Id}}").strip()


def cpu(name):
    fields = run("docker", "exec", name, "cat", "/sys/fs/cgroup/cpu.stat").split()
    return int(dict(zip(fields[::2], fields[1::2]))["usage_usec"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--hirune-image", required=True)
    parser.add_argument("--imgproxy-image", default=IMGPROXY)
    parser.add_argument("--providers", nargs="+", choices=("hirune", "imgproxy"), default=["hirune", "imgproxy"])
    parser.add_argument("--cases", nargs="+", choices=("photo", "progressive", "rotated", "alpha", "animation"),
                        default=["photo", "progressive", "rotated", "alpha", "animation"])
    parser.add_argument("--outputs", nargs="+", type=int, choices=(1, 4), default=[1, 4])
    parser.add_argument("--iterations", type=int, default=20)
    parser.add_argument("--warmup", type=int, default=2)
    parser.add_argument("--concurrency", type=int, default=1)
    parser.add_argument("--fanout", choices=("serial", "parallel"), default="serial")
    parser.add_argument("--profile", choices=("accurate", "preview"), default="accurate")
    parser.add_argument("--decode-mode", choices=("default", "accurate", "preview"), default="default")
    parser.add_argument("--cpus", default="2")
    parser.add_argument("--memory", default="1g")
    parser.add_argument("--fixtures", type=Path, default=ROOT / "target/benchmarks/fixtures")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    assert args.iterations > 0 and args.warmup >= 0 and 1 <= args.concurrency <= 4
    args.fixtures = args.fixtures.resolve()
    args.output = args.output.resolve()
    args.fixtures.mkdir(parents=True, exist_ok=True)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    samples = args.output.parent / (args.output.stem + "-outputs")
    samples.mkdir(parents=True, exist_ok=True)
    print(run("docker", "build", "-f", "benchmarks/imgproxy/client.Dockerfile", "-t", CLIENT, "."), flush=True)
    fixture_manifest = args.fixtures / "manifest.json"
    generator_hash = hashlib.sha256((ROOT / "benchmarks/imgproxy/fixtures.py").read_bytes()).hexdigest()
    if not fixture_manifest.exists() or json.loads(fixture_manifest.read_text())["generator"].get("script_sha256") != generator_hash:
        photo = args.fixtures / "original-iss043e000030.jpg"
        if not photo.exists():
            with urllib.request.urlopen(PHOTO, timeout=120) as response:
                photo.write_bytes(response.read())
        print(run("docker", "run", "--rm", "--network", "none", "--read-only",
                  "--mount", f"type=bind,source={ROOT},target=/work,readonly",
                  "--mount", f"type=bind,source={args.fixtures},target=/fixtures", CLIENT,
                  "python", "benchmarks/imgproxy/fixtures.py", "/fixtures"), flush=True)
    fixture_info = json.loads((args.fixtures / "manifest.json").read_text())
    images = {"hirune": args.hirune_image, "imgproxy": args.imgproxy_image}
    image_ids = {name: ensure_image(images[name]) for name in args.providers}
    info = json.loads(run("docker", "info", "--format", "{{json .}}"))
    report = {"schema_version": 1, "host": platform.platform(), "docker": info["ServerVersion"],
              "docker_cpus": info["NCPU"], "docker_memory_bytes": info["MemTotal"],
              "harness_commit": run("git", "rev-parse", "HEAD").strip(),
              "harness_dirty": bool(run("git", "status", "--porcelain").strip()),
              "fixtures": fixture_info, "images": images, "image_ids": image_ids,
              "settings": vars(args) | {"fixtures": str(args.fixtures), "output": str(args.output)},
              "cpu_scope": "service cgroup delta after warmup through batch-complete handshake, before client validation; excludes origin and client",
              "memory_scope": "fresh service cgroup peak including startup, warmup, measurement, tmpfs and cgroup probes",
              "limits": ["different resize filters and native codecs", "Hirune verifies outputs inside the service",
                         "imgproxy fetches the source for each output URL", "same quality is not equal perceptual quality",
                         "origin and client resource use is excluded from service counters"],
              "order": "alternate provider order for successive case/output pairs", "complete": False, "results": []}
    network = "hirune-imgproxy-bench-" + uuid.uuid4().hex[:12]
    service, client = network + "-service", network + "-client"
    run("docker", "network", "create", "--internal", network)
    scenario_number = 0
    try:
        for case in args.cases:
            for count in args.outputs:
                provider_order = args.providers[::1 if scenario_number % 2 == 0 else -1]
                scenario_number += 1
                for provider in provider_order:
                    row = {"case": case, "outputs": count, "provider": provider, "status": "started"}
                    report["results"].append(row)
                    stage = "start-client"
                    environment = [f"HIRUNE_CONCURRENCY={args.concurrency}"] if provider == "hirune" else [
                        f"IMGPROXY_WORKERS={args.concurrency * (count if args.fanout == 'parallel' else 1)}",
                        "IMGPROXY_MAX_ANIMATION_FRAMES=120", "IMGPROXY_MAX_SRC_RESOLUTION=40",
                        "IMGPROXY_MAX_SRC_FILE_SIZE=33554432", "IMGPROXY_MAX_RESULT_DIMENSION=4096",
                        "IMGPROXY_USE_LINEAR_COLORSPACE=true",
                        "IMGPROXY_STRIP_METADATA=true", "IMGPROXY_KEEP_COPYRIGHT=false", "IMGPROXY_STRIP_COLOR_PROFILE=true",
                        "IMGPROXY_DISABLE_SHRINK_ON_LOAD=" + str(args.profile == "accurate").lower(),
                        "IMGPROXY_WEBP_EFFORT=4", "IMGPROXY_TIMEOUT=60", "IMGPROXY_WRITE_RESPONSE_TIMEOUT=60",
                        "IMGPROXY_ALLOWED_SOURCES=http://bench-client:8090/", "IMGPROXY_LOG_LEVEL=error"]
                    try:
                        run("docker", "run", "-d", "--name", client, "--network", network, "--network-alias", "bench-client",
                            "--read-only", "--cpus", "2", "--memory", "2g",
                            "--mount", f"type=bind,source={ROOT},target=/work,readonly",
                            "--mount", f"type=bind,source={args.fixtures},target=/fixtures,readonly",
                            "--mount", f"type=bind,source={samples},target=/out", CLIENT,
                            "python", "benchmarks/imgproxy/client.py", "serve", "--case", case)
                        stage = "start-service"
                        run("docker", "run", "-d", "--name", service, "--network", network,
                            "--network-alias", "bench-service", "--read-only", "--user", "65532:65532",
                            "--tmpfs", "/tmp:rw,noexec,nosuid,size=512m,mode=1777", "--cap-drop", "ALL",
                            "--security-opt", "no-new-privileges:true", "--memory", args.memory,
                            "--memory-swap", args.memory, "--cpus", args.cpus, "--pids-limit", "64",
                            *[part for value in environment for part in ("--env", value)], images[provider])
                        path = "/health/ready" if provider == "hirune" else "/health"
                        wait = ("import time,urllib.request\nfor attempt in range(200):\n try:\n"
                                f"  urllib.request.urlopen('http://bench-service:8080{path}',timeout=2).read(); break\n"
                                " except OSError: time.sleep(.1)\nelse: raise RuntimeError('service not ready')")
                        run("docker", "exec", client, "python", "-c", wait)
                        service_info = json.loads(run("docker", "exec", client, "python", "-c",
                            "import urllib.request;print(urllib.request.urlopen('http://bench-service:8080/v1/capabilities').read().decode())")) if provider == "hirune" else {
                                "version": run("docker", "run", "--rm", "--network", "none", images[provider], "imgproxy", "version").strip(),
                                "native_threads": "1 set by imgproxy v4.0.20 vips.Init outside Lambda",
                                "libvips_operation_cache": "disabled by imgproxy v4.0.20 vips.Init"}
                        common = ["docker", "exec", client, "python", "benchmarks/imgproxy/client.py", "measure",
                                  "--provider", provider, "--case", case, "--outputs", str(count),
                                  "--concurrency", str(args.concurrency), "--fanout", args.fanout,
                                  "--profile", args.profile, "--decode-mode", args.decode_mode]
                        if args.warmup:
                            stage = "warmup"
                            warmup = json.loads(run(*common, "--iterations", str(args.warmup),
                                                    "--save-outputs", f"/out/{provider}-{count}-warmup"))
                            assert not warmup["errors"], warmup["errors"]
                        stage = "measure"
                        before = cpu(service)
                        process = subprocess.Popen([*common[:2], "-i", *common[2:], "--iterations", str(args.iterations),
                            "--save-outputs", f"/out/{provider}-{count}", "--handshake"], cwd=ROOT, text=True,
                            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                        try:
                            phase = queue.Queue()
                            threading.Thread(target=lambda: phase.put(process.stdout.readline()), daemon=True).start()
                            marker = json.loads(phase.get(timeout=600))
                            assert marker["phase"] == "measured"
                            measured_cpu = (cpu(service) - before) / 1e6
                            stage = "validate"
                            output, error = process.communicate("validate\n", timeout=600)
                            if process.returncode:
                                raise RuntimeError(error)
                            result = json.loads(output)
                        except Exception:
                            process.kill()
                            process.communicate()
                            raise
                        result["cgroup_cpu_seconds"] = measured_cpu
                        result["cgroup_memory_peak_bytes"] = int(run("docker", "exec", service, "cat", "/sys/fs/cgroup/memory.peak"))
                        result["service_info"] = service_info
                        result["environment"] = environment
                        row.update(result)
                        print(f"{provider} {case}/{count}: p50={result['latency_ms']['p50']}ms, successes={result['successes']}/{args.iterations}", flush=True)
                        assert not result["errors"], result["errors"]
                        row["status"] = "success"
                    except Exception as error:
                        row.update(status="failed", failure_stage=stage, failure=repr(error))
                        logs = subprocess.run(["docker", "logs", service], capture_output=True, text=True)
                        row["service_logs"] = logs.stdout + logs.stderr
                        print(row["service_logs"], file=sys.stderr)
                        raise
                    finally:
                        subprocess.run(["docker", "rm", "--force", service, client], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        report["complete"] = True
    finally:
        subprocess.run(["docker", "network", "rm", network], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf8")


if __name__ == "__main__":
    main()
