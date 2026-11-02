"""Compare retained first-iteration outputs after the timed benchmarks finish."""
import argparse
import hashlib
import io
import json
import math
from pathlib import Path
from PIL import Image, ImageChops, ImageStat, __version__

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--results', type=Path, default=Path(__file__).resolve().parents[2] / 'target/benchmarks')
root = parser.parse_args().results.resolve()

def load(name):
    report = json.loads((root / f'{name}.json').read_text())
    assert report['complete']
    assert all(row['status'] == 'success' and not row['errors'] for row in report['results'])
    return report

def rows(report):
    return {(row['case'], row['outputs']): row for row in report['results'] if row['provider'] == 'hirune'}

equal = []
for before, after in [('baseline-accurate', 'candidate-accurate'), ('baseline-full', 'candidate-full')]:
    base, candidate = rows(load(before)), rows(load(after))
    for key in base:
        old, new = base[key], candidate[key]
        assert old['source_sha256'] == new['source_sha256']
        for index, (left, right) in enumerate(zip(old['output_facts'], new['output_facts'], strict=True)):
            assert left['sha256'] == right['sha256'], (key, index, left, right)
            payloads = []
            for name, fact in [(before, left), (after, right)]:
                path = root / f'{name}-outputs' / f'hirune-{key[1]}' / f'{key[0]}-{index}.webp'
                payload = path.read_bytes()
                assert hashlib.sha256(payload).hexdigest() == fact['sha256']
                payloads.append(payload)
            assert payloads[0] == payloads[1]
            equal.append({'baseline_report': before, 'candidate_report': after,
                          'case': key[0], 'outputs': key[1], 'index': index, 'sha256': left['sha256']})

accurate, preview = rows(load('candidate-accurate')), rows(load('candidate-preview'))
quality = []
for key, row in preview.items():
    assert row['source_sha256'] == accurate[key]['source_sha256']
    for index, fact in enumerate(row['output_facts']):
        paths = [root / f'{name}-outputs' / f'hirune-{key[1]}' / f'{key[0]}-{index}.webp'
                 for name in ['candidate-accurate', 'candidate-preview']]
        images = []
        for path, expected in zip(paths, [accurate[key]['output_facts'][index], fact], strict=True):
            raw = path.read_bytes()
            assert hashlib.sha256(raw).hexdigest() == expected['sha256']
            pixels = Image.open(io.BytesIO(raw)).convert('RGBA')
            assert pixels.getextrema()[3] == (255, 255)
            images.append(pixels.convert('RGB'))
        assert images[0].size == images[1].size
        stats = ImageStat.Stat(ImageChops.difference(*images))
        mse = sum(value ** 2 for value in stats.rms) / 3
        quality.append({'case': key[0], 'outputs': key[1], 'index': index,
                        'width': images[0].width, 'height': images[0].height,
                        'accurate_sha256': accurate[key]['output_facts'][index]['sha256'],
                        'preview_sha256': fact['sha256'],
                        'mean_absolute_rgb_code_difference': sum(stats.mean) / 3,
                        'rgb_psnr_db': 10 * math.log10(255 ** 2 / mse) if mse else None})

result = {'pillow': __version__, 'accurate_byte_identical_outputs': equal,
          'preview_comparisons': quality,
          'scope': 'First measured output sample for each size. JPEG outputs are opaque. RGB statistics use decoded sRGB code values.',
          'limits': 'PSNR compares preview with the accurate Hirune output at lossy WebP q85/effort4. This is neither a perceptual quality ranking nor an image-fidelity threshold.'}
(root / 'quality-comparison.json').write_text(json.dumps(result, indent=2) + '\n')
scores = [q['rgb_psnr_db'] for q in quality if q['rgb_psnr_db'] is not None]
print(json.dumps({'accurate_byte_identical_outputs': len(equal), 'preview_samples': len(quality),
                  'preview_mae_min': min(q['mean_absolute_rgb_code_difference'] for q in quality),
                  'preview_mae_max': max(q['mean_absolute_rgb_code_difference'] for q in quality),
                  'preview_psnr_min': min(scores, default=None),
                  'preview_psnr_max': max(scores, default=None)}, indent=2))
