# Image fixtures

The supplied image-processor handoff provided the original fixtures on 2026-10-16.
They include synthetic GIF timelines, manual composited RGBA reference pixels, oriented JPEG/GPS/ICC samples, and libwebp WebP samples.
The original archive's SHA256SUMS verified 147 files before extraction.
The metadata in these files is synthetic test data.

The expected GIF frame delays are 100, 700, and 300 ms.
The disposal fixture adds a fourth delay of 500 ms.
Finite repeat fields of 1 and 3 mean total playback counts of 2 and 4.
Without a loop extension, playback occurs once.
Tests ignore RGB values under zero alpha. Runtime outputs must contain no source metadata.

`p3-color.png` embeds the handoff P3 profile in synthetic RGBA swatches.
Pillow 12.3.0 and LittleCMS 2.19 generated `p3-color-expected.rgba` independently of moxcms, with perceptual rendering intent.
`p3-color-provenance.json` records the generator and profile hash. SHA256SUMS records fixture hashes.
To reproduce the reference, run `python scripts/generate_color_fixture.py` with those library versions.
The RGB tolerance is two code values. Alpha must match exactly. Tests ignore hidden RGB under zero alpha.

`scripts/generate_animation_reference.py` calculates `disposal-small-expected-*.rgba` from the original manual composited displays.
Its separate scalar Python area calculation produces 4×3 reference pixels.
The test checks transparent edges and disposal after resizing. Alpha must match exactly; RGB tolerance is one code value.
Normal CI also runs header mutations and truncations with a fixed seed and bounded inputs.
That test does not replace a long-running native-code fuzz campaign.
