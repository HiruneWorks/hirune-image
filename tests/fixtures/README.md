# Image fixtures

Synthetic GIF timelines, manual composited RGBA expectations, oriented JPEG/GPS/ICC samples, and libwebp WebP samples were retained from the supplied image-processor handoff (2026-10-16). The original archive SHA256SUMS verified 147 files before extraction. These files contain synthetic test metadata.

Expected GIF delays: 100/700/300 ms; disposal: 100/700/300/500 ms. Finite repeat fields 1 and 3 mean total loop counts 2 and 4. Missing loop extension means one playback. RGB under zero alpha is not compared. Runtime outputs must contain no source metadata.
