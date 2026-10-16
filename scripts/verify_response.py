"""Independent v1 multipart binding/structure verifier; codec decoding remains server-side."""
import argparse
from email.parser import BytesParser
import email.policy
import hashlib
import json
from pathlib import Path
import struct

def strict_json(data):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate JSON member")
            result[key] = value
        return result
    return json.loads(data, object_pairs_hook=unique)

def inspect_webp(data):
    assert data[:4] == b"RIFF" and data[8:12] == b"WEBP", "not WebP"
    assert len(data) == int.from_bytes(data[4:8], "little") + 8, "RIFF length/trailing data"
    width = height = None
    delays = []
    loops = None
    cursor = 12
    while cursor < len(data):
        assert cursor + 8 <= len(data)
        kind = data[cursor:cursor+4]
        length = int.from_bytes(data[cursor+4:cursor+8], "little")
        body = data[cursor+8:cursor+8+length]
        assert len(body) == length
        if kind == b"VP8X":
            assert length == 10 and width is None
            width = int.from_bytes(body[4:7], "little") + 1
            height = int.from_bytes(body[7:10], "little") + 1
        elif kind == b"VP8L" and width is None:
            assert length >= 5 and body[0] == 0x2f
            bits = int.from_bytes(body[1:5], "little")
            width, height = (bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1
        elif kind == b"VP8 " and width is None:
            assert length >= 10 and body[3:6] == b"\x9d\x01\x2a"
            width = int.from_bytes(body[6:8], "little") & 0x3fff
            height = int.from_bytes(body[8:10], "little") & 0x3fff
        elif kind == b"ANIM":
            assert length == 6 and loops is None
            loops = int.from_bytes(body[4:6], "little")
        elif kind == b"ANMF":
            assert length >= 16
            delays.append(int.from_bytes(body[12:15], "little"))
        else:
            assert kind in (b"VP8 ", b"VP8L", b"ALPH"), "metadata/unknown output chunk"
        cursor += 8 + length + length % 2
    assert cursor == len(data) and width and height
    assert (loops is not None) == bool(delays)
    assert not delays or len(delays) >= 2 and all(0 < d <= 60000 for d in delays)
    return {"width": width, "height": height, "frame_count": len(delays) or 1,
            "duration_ms": sum(delays), "loop_count": loops if delays else 1}

def verify(payload, source=None, recipe=None):
    assert len(payload) <= 34 << 20, "response budget"
    first = payload.split(b"\r\n", 1)[0]
    assert first.startswith(b"--") and 0 < len(first[2:]) <= 70
    boundary = first[2:]
    terminal = b"\r\n--" + boundary + b"--\r\n"
    end = payload.find(terminal)
    assert end >= 0 and end + len(terminal) == len(payload), "missing boundary/trailing data"
    message = BytesParser(policy=email.policy.default).parsebytes(
        b'Content-Type: multipart/related; boundary="' + boundary + b'"\r\nMIME-Version: 1.0\r\n\r\n' + payload)
    assert not message.defects and message.is_multipart()
    parts = list(message.iter_parts())
    assert 2 <= len(parts) <= 5
    for part in parts:
        assert not part.defects
        for name in ("Content-ID", "Content-Type", "Content-Length"):
            assert len(part.get_all(name, [])) == 1, "duplicate/missing part header"
        assert int(part["Content-Length"]) == len(part.get_payload(decode=True))
    assert str(parts[0]["Content-ID"]) == "<manifest>"
    assert parts[0].get_content_type() == "application/json"
    manifest = strict_json(parts[0].get_payload(decode=True))
    assert set(manifest) == {"contract_version", "processor", "source_sha256", "recipe_sha256", "source", "outputs"}
    assert manifest["contract_version"] == 1
    outputs = manifest["outputs"]
    assert len(parts) == len(outputs) + 1
    assert len({output["id"] for output in outputs}) == len(outputs)
    if source is not None:
        assert manifest["source_sha256"] == hashlib.sha256(source).hexdigest()
        assert manifest["source"]["byte_size"] == len(source)
    if recipe is not None:
        assert manifest["recipe_sha256"] == hashlib.sha256(recipe).hexdigest()
        assert [o["id"] for o in strict_json(recipe)["outputs"]] == [o["id"] for o in outputs]
    aggregate = 0
    for part, output in zip(parts[1:], outputs, strict=True):
        data = part.get_payload(decode=True)
        assert str(part["Content-ID"]) == f"<output-{output['id']}>"
        assert part.get_content_type() == output["media_type"] == "image/webp"
        assert len(data) == output["byte_size"]
        assert hashlib.sha256(data).hexdigest() == output["sha256"]
        facts = inspect_webp(data)
        for key, value in facts.items():
            assert type(output[key]) is int and output[key] == value, "output facts mismatch"
        assert facts["width"] <= 4096 and facts["height"] <= 4096
        assert facts["frame_count"] <= 120 and facts["duration_ms"] <= 60000
        aggregate += len(data)
    assert aggregate <= 32 << 20
    return manifest

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("response", type=Path)
    parser.add_argument("--source", type=Path)
    parser.add_argument("--recipe", type=Path)
    args = parser.parse_args()
    manifest = verify(args.response.read_bytes(),
                      args.source.read_bytes() if args.source else None,
                      args.recipe.read_bytes() if args.recipe else None)
    print(json.dumps({"result": "PASS", "output_count": len(manifest["outputs"]),
                      "source_sha256": manifest["source_sha256"]}))

if __name__ == "__main__":
    main()
