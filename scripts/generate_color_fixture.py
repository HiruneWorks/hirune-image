"""Generate independent reference pixels with LittleCMS. Normal tests use the committed files."""
from pathlib import Path
import hashlib
import io
import json
from PIL import Image, ImageCms, __version__

root = Path(__file__).resolve().parents[1] / "tests/fixtures"
profile = Image.open(root / "p3-orientation6-gps.jpg").info["icc_profile"]
pixels = [(r, g, b, a) for r, g, b in [(0, 0, 0), (255, 255, 255), (180, 40, 70), (40, 180, 70), (40, 70, 180), (128, 128, 128), (255, 0, 0), (0, 255, 0)] for a in (255, 128, 1, 0)]
image = Image.new("RGBA", (8, 4))
image.putdata(pixels)
image.save(root / "p3-color.png", icc_profile=profile)
converted = ImageCms.profileToProfile(image, ImageCms.ImageCmsProfile(io.BytesIO(profile)), ImageCms.createProfile("sRGB"), renderingIntent=0, outputMode="RGBA")
(root / "p3-color-expected.rgba").write_bytes(converted.tobytes())
provenance = {"generator": "scripts/generate_color_fixture.py", "pillow": __version__, "littlecms": ImageCms.core.littlecms_version, "intent": "perceptual (0)", "size": [8, 4], "profile_sha256": hashlib.sha256(profile).hexdigest(), "rgb_tolerance": 2, "alpha_tolerance": 0}
(root / "p3-color-provenance.json").write_text(json.dumps(provenance, indent=2) + "\n", encoding="utf-8", newline="\n")
