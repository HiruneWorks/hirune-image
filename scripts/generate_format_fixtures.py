"""Generate synthetic format fixtures. These helpers are not part of the runtime service."""
from pathlib import Path
import argparse
import base64
import hashlib
import itertools
import json
import struct
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "target" / "fixture-python"))
from PIL import Image
import pillow_heif

def cmyk_profile():
    def fixed(value):
        return struct.pack(">i", round(value * 65536))
    white = b"XYZ " + bytes(4) + b"".join(fixed(x) for x in (0.9642, 1, 0.8249))
    lut = b"mft2" + bytes(4) + bytes((4, 3, 2, 0))
    lut += b"".join(fixed(x) for x in (1, 0, 0, 0, 1, 0, 0, 0, 1))
    lut += struct.pack(">HH", 2, 2) + struct.pack(">HH", 0, 65535) * 4
    for c, m, y, k in itertools.product((0, 1), repeat=4):
        r, g, b = (1-c)*(1-k), (1-m)*(1-k), (1-y)*(1-k)
        xyz = (0.4360747*r + 0.3850649*g + 0.1430804*b,
               0.2225045*r + 0.7168786*g + 0.0606169*b,
               0.0139322*r + 0.0971045*g + 0.7141733*b)
        lut += b"".join(struct.pack(">H", round(x*32768)) for x in xyz)
    lut += struct.pack(">HH", 0, 65535) * 3
    header = bytearray(128)
    header[8:12] = bytes.fromhex("02100000")
    header[12:24] = b"scnrCMYKXYZ "
    header[24:36] = struct.pack(">6H", 2026, 11, 1, 0, 0, 0)
    header[36:40] = b"acsp"
    header[68:80] = b"".join(fixed(x) for x in (0.9642, 1, 0.8249))
    data = bytes(header) + struct.pack(">I", 2)
    data += b"wtpt" + struct.pack(">II", 156, len(white))
    data += b"A2B0" + struct.pack(">II", 156+len(white), len(lut))
    data += white + lut
    return struct.pack(">I", len(data)) + data[4:]

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--ffmpeg", default="ffmpeg")
    args = parser.parse_args()
    fixtures = ROOT / "tests" / "fixtures"
    embedded = base64.b64encode((fixtures/"p3-color.png").read_bytes()).decode("ascii")
    width, height = Image.open(fixtures/"p3-color.png").size
    (fixtures/"p3-embedded.svg").write_text(f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}"><image width="{width}" height="{height}" href="data:image/png;base64,{embedded}"/></svg>\n', encoding="utf8")
    frames = [Image.new("RGBA", (8, 4), color) for color in
              ((255, 0, 0, 255), (0, 255, 0, 128), (0, 0, 255, 255))]
    frames[0].save(fixtures/"colors-animated.avif", save_all=True, append_images=frames[1:],
                   duration=[100, 200, 300], quality=100, speed=10, subsampling="4:4:4")
    frames[0].save(fixtures/"red-static.avif", quality=100, speed=10, subsampling="4:4:4")
    pillow_heif.from_pillow(frames[1]).save(fixtures/"green-alpha.heic", quality=100)
    rgb = frames[0].convert("RGB")
    rgb.convert("CMYK").save(fixtures/"red-cmyk.jpg", quality=100, subsampling=0)
    rgb.convert("CMYK").save(fixtures/"red-cmyk-icc.jpg", quality=100, subsampling=0, icc_profile=cmyk_profile())
    with tempfile.TemporaryDirectory() as scratch:
        # Pillow writes indefinite repetition. Remux the timed color/alpha tracks with three total plays.
        finite = Path(scratch)/"finite.avif"
        subprocess.run([args.ffmpeg, "-hide_banner", "-loglevel", "error", "-y", "-i", str(fixtures/"colors-animated.avif"),
                        "-map", "0:2", "-map", "0:3", "-c", "copy", "-loop", "3", "-f", "avif", str(finite)], check=True)
        (fixtures/"colors-animated.avif").write_bytes(finite.read_bytes())
        for index, frame in enumerate(frames):
            frame.save(Path(scratch)/f"frame{index:02}.png")
        common = [args.ffmpeg, "-hide_banner", "-loglevel", "error", "-y"]
        subprocess.run(common + ["-framerate", "10", "-i", str(Path(scratch)/"frame%02d.png"),
                                 "-frames:v", "3", "-c:v", "libjxl_anim", "-distance", "0",
                                 "-effort", "1", "-f", "image2pipe", str(fixtures/"colors-animated.jxl")], check=True)
        subprocess.run(common + ["-i", str(Path(scratch)/"frame00.png"), "-frames:v", "1",
                                 "-c:v", "libjxl", "-distance", "0", "-effort", "1",
                                 str(fixtures/"red-static.jxl")], check=True)
    names = ["colors-animated.avif", "red-static.avif", "green-alpha.heic", "red-cmyk.jpg",
             "red-cmyk-icc.jpg", "colors-animated.jxl", "red-static.jxl", "p3-embedded.svg"]
    record = {"generator": "scripts/generate_format_fixtures.py", "pillow": Image.__version__,
              "pillow_heif": pillow_heif.__version__,
              "ffmpeg": subprocess.check_output([args.ffmpeg, "-version"], text=True).splitlines()[0],
              "source_colors_rgba": [[255,0,0,255], [0,255,0,128], [0,0,255,255]],
              "avif_frame_delays_ms": [100,200,300], "avif_total_plays": 3,
              "jxl_frame_delays_ms": [100,100,100], "jxl_total_plays": 0,
              "license": "CC0-1.0", "sha256": {name: hashlib.sha256((fixtures/name).read_bytes()).hexdigest() for name in names}}
    (fixtures/"formats-provenance.json").write_text(json.dumps(record, indent=2)+"\n", encoding="utf8")
    print(json.dumps(record, indent=2))

if __name__ == "__main__":
    main()
