# Image fixtures

Synthetic GIF timelines, manual composited RGBA expectations, oriented JPEG/GPS/ICC samples, and libwebp WebP samples were retained from the supplied image-processor handoff (2026-10-16). The original archive SHA256SUMS verified 147 files before extraction. These files contain synthetic test metadata.

Expected GIF delays: 100/700/300 ms; disposal: 100/700/300/500 ms. Finite repeat fields 1 and 3 mean total loop counts 2 and 4. Missing loop extension means one playback. RGB under zero alpha is not compared. Runtime outputs must contain no source metadata.

`p3-color.png` embeds the handoff P3 profile on synthetic RGBA swatches. `p3-color-expected.rgba` was produced independently by Pillow 12.3.0 / LittleCMS 2.19 with perceptual intent, not moxcms. The generator and hashes are recorded in `p3-color-provenance.json` and SHA256SUMS. Run `python scripts/generate_color_fixture.py` with those versions to reproduce it. RGB tolerance is two code values; alpha is exact; hidden RGB at zero alpha is ignored.

`disposal-small-expected-*.rgba` uses a separate scalar Python integration over the original manual composited displays. The 4×3 output checks transparent edges and disposal after resizing, with exact alpha and one RGB code-value tolerance. Generator: `scripts/generate_animation_reference.py`. Header parsers also run a fixed-seed bounded mutation/truncation corpus in normal CI; this is not a long-running native-code fuzz campaign.
