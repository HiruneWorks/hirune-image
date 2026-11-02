"""Generate the synthetic progressive JPEG used by reduced-decode regression tests."""
import hashlib
from pathlib import Path

from PIL import Image

root = Path(__file__).resolve().parents[1]
pixels = Image.new("RGB", (257, 131))
colors = [(220, 20, 20), (20, 220, 20), (20, 20, 220), (220, 220, 20)]
pixels.putdata([colors[(y >= 65) * 2 + (x >= 128)] for y in range(131) for x in range(257)])
path = root / "tests/fixtures/preview-progressive.jpg"
pixels.save(path, quality=94, subsampling=0, progressive=True)
print(path.name, hashlib.sha256(path.read_bytes()).hexdigest())

thin = Image.new("RGB", (257, 1))
thin.putdata([(x % 256, 80, 150) for x in range(257)])
path = root / "tests/fixtures/preview-progressive-thin.jpg"
thin.save(path, quality=94, subsampling=0, progressive=True)
print(path.name, hashlib.sha256(path.read_bytes()).hexdigest())
