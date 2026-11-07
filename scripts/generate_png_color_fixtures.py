"""Generate synthetic PNG color regressions and an independent LittleCMS reference."""
import argparse
import ctypes
import ctypes.util
import hashlib
import json
from pathlib import Path
import struct
import zlib


def chunk(kind, data):
    return (struct.pack('>I', len(data)) + kind + data
            + struct.pack('>I', zlib.crc32(kind + data)))


def png(pixels, metadata):
    return (b'\x89PNG\r\n\x1a\n'
            + chunk(b'IHDR', struct.pack('>IIBBBBB', len(pixels) // 4, 1, 8, 6, 0, 0, 0))
            + metadata + chunk(b'IDAT', zlib.compress(b'\0' + pixels)) + chunk(b'IEND', b''))


class XyY(ctypes.Structure):
    _fields_ = [('x', ctypes.c_double), ('y', ctypes.c_double), ('Y', ctypes.c_double)]


class Primaries(ctypes.Structure):
    _fields_ = [('red', XyY), ('green', XyY), ('blue', XyY)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--lcms', default=ctypes.util.find_library('lcms2'))
    args = parser.parse_args()
    if not args.lcms:
        parser.error('install LittleCMS 2 or provide --lcms with its shared library path')
    library = ctypes.CDLL(args.lcms)
    functions = {
        'cmsGetEncodedCMMversion': (ctypes.c_uint32, []),
        'cmsBuildGamma': (ctypes.c_void_p, [ctypes.c_void_p, ctypes.c_double]),
        'cmsCreateRGBProfile': (ctypes.c_void_p, [ctypes.POINTER(XyY), ctypes.POINTER(Primaries), ctypes.POINTER(ctypes.c_void_p)]),
        'cmsCreate_sRGBProfile': (ctypes.c_void_p, []),
        'cmsCreateTransform': (ctypes.c_void_p, [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32]),
        'cmsDoTransform': (None, [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_uint32]),
        'cmsDeleteTransform': (None, [ctypes.c_void_p]),
        'cmsCloseProfile': (ctypes.c_int, [ctypes.c_void_p]),
        'cmsFreeToneCurve': (None, [ctypes.c_void_p]),
    }
    for name, (result, arguments) in functions.items():
        function = getattr(library, name)
        function.restype = result
        function.argtypes = arguments
    root = Path(__file__).resolve().parents[1] / 'tests/fixtures'
    standard = [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000]
    sample = bytes([64, 128, 192, 128])
    artifacts = {
        'standard-srgb-calibration.png': png(sample, chunk(b'gAMA', struct.pack('>I', 45455)) + chunk(b'cHRM', struct.pack('>8I', *standard))),
        'linear-gamma.png': png(sample, chunk(b'gAMA', struct.pack('>I', 100000))),
    }
    d50 = [34567, 35850, *standard[2:]]
    gamma = 55000
    pixels = bytes([180, 40, 70, 255, 40, 180, 70, 128, 40, 70, 180, 1, 128, 128, 128, 0])
    artifacts['d50-calibration.png'] = png(pixels, chunk(b'gAMA', struct.pack('>I', gamma)) + chunk(b'cHRM', struct.pack('>8I', *d50)))
    white = XyY(d50[0] / 100000, d50[1] / 100000, 1)
    primaries = Primaries(*(XyY(d50[index] / 100000, d50[index + 1] / 100000, 1) for index in (2, 4, 6)))
    curve = library.cmsBuildGamma(None, 100000 / gamma)
    source = library.cmsCreateRGBProfile(ctypes.byref(white), ctypes.byref(primaries), (ctypes.c_void_p * 3)(curve, curve, curve))
    destination = library.cmsCreate_sRGBProfile()
    rgba8 = (4 << 16) | (3 << 3) | (1 << 7) | 1
    transform = library.cmsCreateTransform(source, rgba8, destination, rgba8, 0, 0x04000000)
    if not all((curve, source, destination, transform)):
        raise RuntimeError('LittleCMS reference construction failed')
    try:
        result = ctypes.create_string_buffer(len(pixels))
        library.cmsDoTransform(transform, ctypes.create_string_buffer(pixels), result, len(pixels) // 4)
        artifacts['d50-calibration-expected.rgba'] = result.raw
    finally:
        library.cmsDeleteTransform(transform)
        library.cmsCloseProfile(destination)
        library.cmsCloseProfile(source)
        library.cmsFreeToneCurve(curve)
    provenance = {
        'generator': 'scripts/generate_png_color_fixtures.py',
        'littlecms_encoded_version': library.cmsGetEncodedCMMversion(),
        'intent': 'perceptual (0)', 'gamma': gamma, 'chromaticities': d50,
        'rgb_tolerance': 2, 'alpha_tolerance': 0,
        'sha256': {name: hashlib.sha256(data).hexdigest() for name, data in artifacts.items()},
    }
    artifacts['png-color-provenance.json'] = (json.dumps(provenance, indent=2) + '\n').encode()
    for name, data in artifacts.items():
        (root / name).write_bytes(data)
    sums = root / 'SHA256SUMS'
    lines = [line for line in sums.read_text().splitlines() if line.split('  ', 1)[1] not in artifacts]
    lines.extend(f'{hashlib.sha256(data).hexdigest()}  {name}' for name, data in artifacts.items())
    sums.write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps(provenance, indent=2))


if __name__ == '__main__':
    main()
