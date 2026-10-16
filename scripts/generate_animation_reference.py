"""Scalar area integration from hand-authored composited RGBA fixtures, without Rust codecs."""
import json
import math
from pathlib import Path
import struct
ROOT = Path(__file__).resolve().parents[1] / "tests/fixtures"
source = (ROOT / "disposal.gif").read_bytes()
width, height = struct.unpack_from("<HH", source, 6)
target_width, target_height = 4, 3
def linear(value):
    value /= 255
    return value / 12.92 if value <= .04045 else ((value + .055) / 1.055) ** 2.4
def srgb(value):
    return value * 12.92 if value <= .0031308 else 1.055 * value ** (1 / 2.4) - .055
for frame in range(4):
    raw = (ROOT / f"disposal-expected-{frame}.rgba").read_bytes()
    assert len(raw) == width * height * 4
    output = bytearray()
    for ty in range(target_height):
        for tx in range(target_width):
            weights = []
            for y in range(height):
                for x in range(width):
                    overlap_x = max(0, min((tx + 1) * width / target_width, x + 1) - max(tx * width / target_width, x))
                    overlap_y = max(0, min((ty + 1) * height / target_height, y + 1) - max(ty * height / target_height, y))
                    if overlap_x * overlap_y:
                        p = raw[(y * width + x) * 4:][:4]
                        weights.append((p, overlap_x * overlap_y))
            alpha = math.fsum(p[3] / 255 * w for p, w in weights)
            rgb = [math.fsum(linear(p[c]) * p[3] / 255 * w for p, w in weights) / alpha if alpha else 0 for c in range(3)]
            output.extend([max(0, min(255, math.floor(srgb(v) * 255 + .5))) for v in rgb] + [math.floor(alpha / (width * height / target_width / target_height) * 255 + .5)])
    (ROOT / f"disposal-small-expected-{frame}.rgba").write_bytes(output)
(ROOT / "disposal-small-provenance.json").write_text(json.dumps({"generator": "scripts/generate_animation_reference.py", "source": "manual disposal-expected-*.rgba", "size": [4, 3], "rgb_tolerance": 1, "alpha_tolerance": 0}, indent=2) + "\n", encoding="utf-8")
