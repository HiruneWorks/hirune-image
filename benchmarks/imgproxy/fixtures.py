"""Prepare fixed benchmark sources outside all measurement intervals."""
import argparse
import hashlib
import json
from pathlib import Path

import PIL
from PIL import Image, ImageDraw, ImageOps, features

PHOTO_SHA256 = "a7f2ce1bf0b547d76fafe930926a874d3c48eb6025ca172c40c0d090581c4336"
PHOTO_URL = "https://images-assets.nasa.gov/image/iss043e000030/iss043e000030~orig.jpg"


def prepare(directory):
    original = directory / "original-iss043e000030.jpg"
    assert hashlib.sha256(original.read_bytes()).hexdigest() == PHOTO_SHA256, "photo changed"
    photo = Image.open(original).convert("RGB")
    assert photo.width >= 4000 and photo.height >= 2800, "do not enlarge the photo fixture"
    photo = ImageOps.fit(photo, (4000, 2800), Image.Resampling.LANCZOS)
    marks = ImageDraw.Draw(photo)
    marks.rectangle((0, 0, 319, 319), fill=(240, 20, 20))
    marks.rectangle((3680, 2480, 3999, 2799), fill=(20, 220, 80))
    entries = {}

    def add(name, path, image, orientation=1, delays=None, loops=1):
        oriented = ImageOps.exif_transpose(image)
        entries[name] = {"file": path.name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                         "bytes": path.stat().st_size, "display_size": list(oriented.size),
                         "orientation": orientation, "delays_ms": delays or [], "total_plays": loops}

    for name, progressive in [("photo", False), ("progressive", True)]:
        path = directory / (name + ".jpg")
        photo.save(path, quality=93, subsampling=0, progressive=progressive)
        assert not Image.open(path).info.get("icc_profile")
        add(name, path, photo)
    rotated = photo.resize((3997, 2797), Image.Resampling.LANCZOS).transpose(Image.Transpose.ROTATE_90)
    exif = Image.Exif()
    exif[274] = 6
    path = directory / "rotated.jpg"
    rotated.save(path, quality=93, subsampling=0, exif=exif)
    add("rotated", path, Image.open(path), orientation=6)

    alpha = ImageOps.fit(photo, (1537, 1025), Image.Resampling.LANCZOS).convert("RGBA")
    mask = Image.new("L", alpha.size)
    mask.putdata([0 if x < alpha.width // 3 else 255 if x >= alpha.width * 2 // 3
                  else (x - alpha.width // 3) * 255 // (alpha.width // 3)
                  for y in range(alpha.height) for x in range(alpha.width)])
    alpha.putalpha(mask)
    path = directory / "alpha.png"
    alpha.save(path)
    add("alpha", path, alpha)

    frames, delays = [], []
    for index in range(60):
        frame = Image.new("RGB", (638, 362), (32, 80, 128))
        draw = ImageDraw.Draw(frame)
        draw.rectangle((index * 19 % 510, 140, index * 19 % 510 + 127, 300), fill=(240, 80, 16))
        for bit in range(6):
            color = 255 if index & (1 << bit) else 0
            draw.rectangle((bit * 638 // 6, 0, (bit + 1) * 638 // 6 - 1, 95), fill=(color,) * 3)
        frames.append(frame)
        delays.append([40, 80, 120][index % 3])
    path = directory / "animation.gif"
    frames[0].save(path, save_all=True, append_images=frames[1:], duration=delays,
                   loop=2, disposal=2, optimize=False)
    add("animation", path, frames[0], delays=delays, loops=3)
    report = {"schema_version": 1, "photo": {"source": PHOTO_URL, "credit": "NASA / Expedition 43 crew",
              "nasa_id": "iss043e000030", "original_sha256": PHOTO_SHA256,
              "usage_guidelines": "https://www.nasa.gov/nasa-brand-center/images-and-media/",
              "changes": "crop, resize, re-encode, orientation markers, EXIF variant, synthetic alpha; no endorsement"},
              "generator": {"pillow": PIL.__version__, "jpeg": features.version_codec("jpg"),
                            "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},
              "cases": entries}
    (directory / "manifest.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    prepare(parser.parse_args().directory)
