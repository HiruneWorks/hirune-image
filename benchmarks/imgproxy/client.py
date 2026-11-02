"""Serve fixed source bytes and measure complete image-variant work units."""
import argparse
import base64
from concurrent.futures import ThreadPoolExecutor
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import io
import json
import math
from pathlib import Path
import statistics
import struct
import sys
import threading
import time
import urllib.request

from PIL import Image, ImageChops, ImageOps, ImageStat

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
from verify_response import inspect_webp, verify


def fetch(url, data=None, content_type=None):
    request = urllib.request.Request(url, data=data,
                                    headers={"Content-Type": content_type} if content_type else {})
    with urllib.request.urlopen(request, timeout=120) as response:
        return response.read()


def serve(args):
    manifest = json.loads((args.fixtures / "manifest.json").read_text())
    case = manifest["cases"][args.case]
    source = (args.fixtures / case["file"]).read_bytes()
    assert hashlib.sha256(source).hexdigest() == case["sha256"]
    lock = threading.Lock()
    stats = {"requests": 0, "bytes": 0}

    class Origin(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def do_GET(self):
            if self.path == "/source":
                with lock:
                    stats["requests"] += 1
                    stats["bytes"] += len(source)
                body, kind = source, "application/octet-stream"
            elif self.path in ("/stats", "/reset"):
                with lock:
                    if self.path == "/reset":
                        stats.update(requests=0, bytes=0)
                    body = json.dumps(stats).encode()
                kind = "application/json"
            else:
                self.send_error(404)
                return
            self.send_response(200)
            self.send_header("Content-Type", kind)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *_):
            pass

    ThreadingHTTPServer(("0.0.0.0", 8090), Origin).serve_forever()


def inside(width, height, max_width, max_height):
    if width <= max_width and height <= max_height:
        return width, height
    if max_width * height <= max_height * width:
        return max_width, max(1, height * max_width // width)
    return max(1, width * max_height // height), max_height


def delays(data):
    result, cursor = [], 12
    while cursor < len(data):
        length = struct.unpack_from("<I", data, cursor + 4)[0]
        if data[cursor:cursor + 4] == b"ANMF":
            result.append(int.from_bytes(data[cursor + 20:cursor + 23], "little"))
        cursor += 8 + length + length % 2
    return result


def inspect_output(data, provider):
    # imgproxy can emit generated EXIF even with source metadata stripping enabled.
    # Inspect its original pixels separately; do not apply Hirune's metadata-free contract to it.
    if provider == "hirune":
        return inspect_webp(data), []
    assert data[:4] == b"RIFF" and data[8:12] == b"WEBP"
    assert len(data) == int.from_bytes(data[4:8], "little") + 8
    chunks, metadata, cursor = [], [], 12
    while cursor < len(data):
        assert cursor + 8 <= len(data)
        length = int.from_bytes(data[cursor + 4:cursor + 8], "little")
        end = cursor + 8 + length + length % 2
        assert end <= len(data)
        kind = data[cursor:cursor + 4]
        chunk = bytearray(data[cursor:end])
        if kind in (b"EXIF", b"ICCP", b"XMP "):
            metadata.append({"kind": kind.decode("ascii"), "bytes": length})
        else:
            if kind == b"VP8X":
                assert length == 10
                chunk[8] &= ~(0x20 | 0x08 | 0x04)
            chunks.append(chunk)
        cursor = end
    body = b"WEBP" + b"".join(chunks)
    return inspect_webp(b"RIFF" + struct.pack("<I", len(body)) + body), metadata


def measure(args):
    case = json.loads((args.fixtures / "manifest.json").read_text())["cases"][args.case]
    source = (args.fixtures / case["file"]).read_bytes()
    sides = [160, 320, 640, 960, 1280, 1920, 2560, 4096] if args.workload == "full" else [384, 192, 96, 48][:args.outputs]
    boxes = [(side, 4096 if args.workload == "full" else side) for side in sides]
    quality = 90 if args.workload == "full" else 85
    sizes = [inside(*case["display_size"], *box) for box in boxes]
    recipe = {"contract_version": 1, "animation_policy": "preserve", "outputs": [
        {"id": f"size_{width}", "resize": {"fit": "inside", "max_width": width, "max_height": height},
         "encoding": {"format": "webp", "mode": "lossy", "quality": quality, "effort": 4}}
        for width, height in boxes]}
    if args.decode_mode != "default":
        recipe["decode_mode"] = args.decode_mode
    raw_recipe = json.dumps(recipe, separators=(",", ":")).encode()
    boundary = "imgproxy-comparison"
    body = (f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="fixture"\r\n\r\n'.encode()
            + source + f'\r\n--{boundary}\r\nContent-Disposition: form-data; name="recipe"\r\nContent-Type: application/json\r\n\r\n'.encode()
            + raw_recipe + f"\r\n--{boundary}--\r\n".encode())
    encoded = base64.urlsafe_b64encode(b"http://bench-client:8090/source").decode().rstrip("=")
    urls = [f"{args.base}/unsafe/rs:force:{w}:{h}:0/q:{quality}/{encoded}.webp" for w, h in sizes]
    attempt_lock = threading.Lock()
    attempts = 0

    def processing_fetch(url, data=None, content_type=None):
        nonlocal attempts
        with attempt_lock:
            attempts += 1
        return fetch(url, data, content_type)

    def send(_):
        started = time.perf_counter()
        try:
            if args.provider == "hirune":
                payloads = [processing_fetch(args.base + "/v1/images/process", body, f"multipart/form-data; boundary={boundary}")]
            elif args.fanout == "parallel":
                with ThreadPoolExecutor(max_workers=len(urls)) as pool:
                    payloads = list(pool.map(processing_fetch, urls))
            else:
                payloads = [processing_fetch(url) for url in urls]
            return {"latency_ms": (time.perf_counter() - started) * 1000, "payloads": payloads}
        except Exception as error:
            return {"latency_ms": (time.perf_counter() - started) * 1000, "error": repr(error)}

    fetch("http://127.0.0.1:8090/reset")
    started = time.perf_counter()
    with ThreadPoolExecutor(max_workers=args.concurrency) as pool:
        results = list(pool.map(send, range(args.iterations)))
    elapsed = time.perf_counter() - started
    if args.handshake:
        print(json.dumps({"phase": "measured"}), flush=True)
        assert sys.stdin.readline().strip() == "validate", "measurement controller disconnected"
    origin = json.loads(fetch("http://127.0.0.1:8090/stats"))
    reference = ImageOps.exif_transpose(Image.open(io.BytesIO(source))).convert("RGBA")
    references = [reference.resize(size, Image.Resampling.BOX) for size in sizes]
    output_facts, errors, times = [], [], []
    for iteration, result in enumerate(results):
        if "error" in result:
            errors.append(result["error"])
            continue
        try:
            if args.provider == "hirune":
                verify(result["payloads"][0], source, raw_recipe)
                from email.parser import BytesParser
                import email.policy
                payload = result["payloads"][0]
                header = b'Content-Type: multipart/related; boundary="' + payload.split(b"\r\n")[0][2:] + b'"\r\n\r\n'
                parts = list(BytesParser(policy=email.policy.default).parsebytes(header + payload).iter_parts())
                buffers = [part.get_payload(decode=True) for part in parts[1:]]
            else:
                buffers = result["payloads"]
            facts = []
            for index, (buffer, size) in enumerate(zip(buffers, sizes, strict=True)):
                if iteration == 0 and args.save_outputs:
                    args.save_outputs.mkdir(parents=True, exist_ok=True)
                    (args.save_outputs / f"{args.case}-{index}.webp").write_bytes(buffer)
                actual, metadata = inspect_output(buffer, args.provider)
                assert (actual["width"], actual["height"]) == size
                assert actual["duration_ms"] == sum(case["delays_ms"])
                assert actual["loop_count"] == case["total_plays"]
                assert delays(buffer) == case["delays_ms"], "animation frame timing changed"
                image = Image.open(io.BytesIO(buffer))
                assert image.n_frames == (len(case["delays_ms"]) or 1)
                for frame in range(image.n_frames):
                    image.seek(frame)
                    rgba = image.convert("RGBA")
                    if args.case == "alpha":
                        assert rgba.getpixel((size[0] // 6, size[1] // 2))[3] <= 2, "transparent region lost"
                        assert rgba.getpixel((size[0] * 5 // 6, size[1] // 2))[3] >= 253, "opaque region lost"
                    if args.case in ("photo", "progressive", "rotated"):
                        mark_x = max(0, int(160 * size[0] / case["display_size"][0]))
                        mark_y = max(0, int(160 * size[1] / case["display_size"][1]))
                        red = rgba.getpixel((mark_x, mark_y))
                        green = rgba.getpixel((size[0] - mark_x - 1, size[1] - mark_y - 1))
                        assert red[0] > 180 and red[1] < 75 and green[1] > 160 and green[0] < 85, "orientation or pixels changed"
                    if args.case == "animation":
                        for bit in range(6):
                            point = (min(size[0] - 1, int((bit + .5) * size[0] / 6)), max(0, int(40 * size[1] / 362)))
                            sample = rgba.getpixel(point)
                            assert (min(sample[:3]) > 180) if frame & (1 << bit) else (max(sample[:3]) < 75), "animation order changed"
                fact = {**actual, "bytes": len(buffer), "sha256": hashlib.sha256(buffer).hexdigest(),
                        "metadata_chunks": metadata}
                if not case["delays_ms"]:
                    def on_black(pixels):
                        return Image.alpha_composite(Image.new("RGBA", size, (0, 0, 0, 255)), pixels).convert("RGB")
                    difference = ImageChops.difference(on_black(rgba), on_black(references[index]))
                    fact["mae_against_srgb_box_reference"] = statistics.mean(ImageStat.Stat(difference).mean)
                    assert fact["mae_against_srgb_box_reference"] < 35, "invalid visual output"
                facts.append(fact)
            if not output_facts:
                output_facts = facts
            times.append(result["latency_ms"])
        except (AssertionError, ValueError, OSError, KeyError) as error:
            errors.append("invalid response: " + repr(error))
    times.sort()
    print(json.dumps({"case": args.case, "provider": args.provider, "outputs": args.outputs,
        "profile": args.profile, "decode_mode": args.decode_mode, "fanout": args.fanout,
        "workload": args.workload, "quality": quality, "effort": 4, "unique_output_dimensions": len(set(sizes)),
        "source_sha256": case["sha256"], "source_bytes": len(source), "source_facts": case,
        "iterations": args.iterations, "concurrency": args.concurrency, "successes": len(times), "errors": errors,
        "work_units_per_second": len(times) / elapsed, "batch_seconds": elapsed,
        "latency_ms": {"p50": statistics.median(times) if times else None,
                       "p95": times[math.ceil(len(times) * .95) - 1] if len(times) >= 20 else None},
        "latencies_ms": times,
        "planned_http_processing_requests": args.iterations * (1 if args.provider == "hirune" else len(urls)),
        "http_processing_requests": attempts,
        "origin": origin, "output_facts": output_facts}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("serve", "measure"))
    parser.add_argument("--fixtures", type=Path, default=Path("/fixtures"))
    parser.add_argument("--case", required=True)
    parser.add_argument("--provider", choices=("hirune", "imgproxy"))
    parser.add_argument("--base", default="http://bench-service:8080")
    parser.add_argument("--iterations", type=int, default=20)
    parser.add_argument("--outputs", type=int, choices=(1, 4, 8), default=1)
    parser.add_argument("--workload", choices=("thumbnails", "full"), default="thumbnails")
    parser.add_argument("--concurrency", type=int, default=1)
    parser.add_argument("--fanout", choices=("serial", "parallel"), default="serial")
    parser.add_argument("--profile", choices=("accurate", "preview"), default="accurate")
    parser.add_argument("--decode-mode", choices=("default", "accurate", "preview"), default="default")
    parser.add_argument("--save-outputs", type=Path)
    parser.add_argument("--handshake", action="store_true")
    args = parser.parse_args()
    assert args.iterations > 0 and args.concurrency > 0
    serve(args) if args.action == "serve" else measure(args)
