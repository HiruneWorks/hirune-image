"""Build and verify a private, resource-limited local Linux image. No publication."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import urllib.error
import urllib.request
import uuid
from verify_response import verify

ROOT = Path(__file__).resolve().parents[1]
IMAGE = os.environ.get("HIRUNE_TEST_IMAGE", "hirune-image:local")
CLIENT_IMAGE = "python:3.14-slim"

def command(*args, capture=False):
    return subprocess.run(args, cwd=ROOT, check=True, text=True,
                          stdout=subprocess.PIPE if capture else None).stdout

def request(base, path, body=None, content_type=None):
    req = urllib.request.Request(base + path, data=body)
    if content_type:
        req.add_header("Content-Type", content_type)
    return urllib.request.urlopen(req, timeout=15)

def http_checks(base):
    for attempt in range(100):
        try:
            with request(base, "/health/ready") as response:
                assert response.status == 204
            break
        except (OSError, urllib.error.URLError):
            time.sleep(0.1)
    else:
        raise AssertionError("container did not become ready")
    with request(base, "/v1/capabilities") as response:
        capability = json.load(response)
    assert capability["native_resource_fence"] == "rlimit_as_and_cpu"
    assert capability["limits"]["concurrency"] == 1
    for filename in ["finite1.gif", "disposal.gif", "p3-orientation6-gps.jpg", "rust-finite1-lossless.webp"]:
        for mode in ["lossy", "lossless"]:
            encoding = {"format": "webp", "mode": mode, "effort": 0}
            if mode == "lossy":
                encoding["quality"] = 85
            recipe = {"contract_version": 1, "animation_policy": "preserve", "outputs": [
                {"id": "large", "resize": {"fit": "inside", "max_width": 4096, "max_height": 4096}, "encoding": encoding},
                {"id": "small", "resize": {"fit": "inside", "max_width": 16, "max_height": 16}, "encoding": encoding}]}
            raw_recipe = json.dumps(recipe, separators=(",", ":")).encode()
            source = (ROOT / "tests/fixtures" / filename).read_bytes()
            boundary = "verify-" + uuid.uuid4().hex
            body = (f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"ignored\"\r\nContent-Type: application/octet-stream\r\n\r\n".encode()
                    + source + f"\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"recipe\"\r\nContent-Type: application/json\r\n\r\n".encode()
                    + raw_recipe + f"\r\n--{boundary}--\r\n".encode())
            with request(base, "/v1/images/process", body, f"multipart/form-data; boundary={boundary}") as response:
                verify(response.read(), source, raw_recipe)
    with request(base, "/health/live") as response:
        assert response.status == 204
    return capability

def main():
    if len(sys.argv) == 3 and sys.argv[1] == "--client":
        print(json.dumps(http_checks(sys.argv[2])))
        return
    name = "hirune-image-verify-" + uuid.uuid4().hex[:12]
    if os.environ.get("HIRUNE_SKIP_BUILD") != "1":
        command("docker", "build", "--file", "compose/production/app/Dockerfile", "--tag", IMAGE, ".")
    command("docker", "pull", CLIENT_IMAGE)
    command("docker", "network", "create", "--internal", name)
    try:
        command("docker", "run", "--detach", "--name", name, "--network", name,
                "--read-only", "--tmpfs", "/tmp:rw,noexec,nosuid,size=512m,mode=1777",
                "--cap-drop", "ALL", "--security-opt", "no-new-privileges:true",
                "--memory", "1g", "--memory-swap", "1g", "--cpus", "2", "--pids-limit", "64", IMAGE)
        capability = json.loads(command("docker", "run", "--rm", "--network", name, "--read-only",
                      "--mount", f"type=bind,source={ROOT},target=/work,readonly", "--workdir", "/work",
                      "--env", "PYTHONDONTWRITEBYTECODE=1", CLIENT_IMAGE,
                      "python", "scripts/verify_container.py", "--client", f"http://{name}:8080", capture=True))
        uid = command("docker", "exec", name, "id", "-u", capture=True).strip()
        assert uid != "0"
        assert command("docker", "exec", name, "sh", "-c", "find /tmp -mindepth 1 -maxdepth 1 | wc -l", capture=True).strip() == "0"
        peak = int(command("docker", "exec", name, "cat", "/sys/fs/cgroup/memory.peak", capture=True))
        assert peak < 1 << 30
        state = json.loads(command("docker", "inspect", name, capture=True))[0]["State"]
        assert state["Running"] and not state["OOMKilled"]
        print(json.dumps({"result": "PASS", "http_cases": 8, "uid": uid,
                          "cgroup_memory_peak_bytes": peak, "container_limit_bytes": 1 << 30,
                          "processor": capability["processor"]}, indent=2))
    finally:
        subprocess.run(["docker", "rm", "--force", name], check=False, stdout=subprocess.DEVNULL)
        subprocess.run(["docker", "network", "rm", name], check=False, stdout=subprocess.DEVNULL)

if __name__ == "__main__":
    main()
