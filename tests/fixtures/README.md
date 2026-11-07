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

## PNG color regressions

`standard-srgb-calibration.png` has standard `gAMA/cHRM` values and no `sRGB` or `iCCP` chunk.
`linear-gamma.png` declares linear RGB samples.
`d50-calibration.png` combines a D50 white point, standard RGB primaries, and gamma `55000`.
LittleCMS 2.16 generated `d50-calibration-expected.rgba` with perceptual intent and exact alpha copying.
`png-color-provenance.json` records the inputs, CMS version, hashes, and tolerances.
Run `python scripts/generate_png_color_fixtures.py` with a LittleCMS 2 shared library to regenerate them.
Normal tests read the committed reference and need no LittleCMS installation.
These synthetic fixtures are dedicated to the public domain under CC0.

## Input format references

`formats-provenance.json` records the new synthetic HEIC, AVIF, JPEG XL, CMYK JPEG, and embedded SVG references.
The source frames contain uniform red, green at alpha 128, and blue pixels.
The AVIF animation has delays of 100, 200, and 300 ms and three total plays.
The JPEG XL animation has three 100 ms frames and an infinite loop.
`red-cmyk-icc.jpg` embeds a synthetic CMYK-to-XYZ LUT profile with defined RGB corner colors.
`p3-embedded.svg` embeds the existing independent P3 color fixture.
SVG pixel checks compare visible premultiplied color because its renderer uses an 8-bit premultiplied surface.

Run `python scripts/generate_format_fixtures.py` from the repository root to regenerate these files.
The recorded generator uses Pillow 13.0.0, pillow-heif 1.8.1, and an FFmpeg build with `libjxl_anim`.
Normal tests read the committed files. They need none of those generation tools.
Update `SHA256SUMS` and the provenance record after an intentional regeneration.
The service does not execute FFmpeg.

## JPEG preview regression

`preview-progressive.jpg` is a synthetic 257×131 progressive RGB JPEG with four distinct color quadrants.
`preview-progressive-thin.jpg` is a 257×1 progressive RGB JPEG. It checks padded coefficient memory.
Generate both with `scripts/generate_preview_fixture.py` and Pillow 13.0.0.
Tests add minimal EXIF records and check all eight transforms, original geometry, accurate fallbacks, and resource limits.
These generated fixtures are dedicated to the public domain under CC0.
