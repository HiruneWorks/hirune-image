# Image processing v1

## Request

Send `POST /v1/images/process` with `multipart/form-data`.
Include exactly two parts: `file` and `recipe`. Either part can come first.
The recipe must contain UTF-8 JSON and use `Content-Type: application/json`.
The service enforces byte limits on the full request and on each part.

The service detects the image format from the file bytes.
It does not use the submitted filename or MIME type to select a decoder or filesystem path.
It rejects the following input:

- Unknown or duplicate multipart parts.
- Unknown or duplicate JSON fields.
- Duplicate output IDs.
- Invalid enum values or option combinations.
- Data after the final multipart boundary.

## Recipe

Use the [recipe schema](../../contracts/image-recipe-v1.schema.json) and [example](../../contracts/examples/image-recipe-v1.json).
Set `contract_version` to `1`.
Supply outputs as an ordered array. Each caller-defined ID must match `^[a-z][a-z0-9_]{0,31}$`.
The operator sets the maximum output count; the default is four.

Set resize fit to `inside`. The service never enlarges an image.
The operator sets the maximum requested side length; the default is 4096 pixels.
The fixed WebP maximum is 16383 pixels.

For lossy WebP, set quality from 0 to 100 and effort from 0 to 6.
For lossless WebP, set effort from 0 to 6. The service rejects a quality field in lossless mode.
Effort sets the native libwebp method.
The worker encodes outputs in sequence. The request succeeds only if every output succeeds.

## Response

A successful request returns `200 multipart/related`.
The first part is a JSON manifest with `Content-ID: <manifest>`.
One WebP part follows for each output, in recipe order. Its Content-ID is `<output-ID>`.
Every part has `Content-Type` and `Content-Length` headers.

The manifest records SHA-256 of the exact source bytes and exact UTF-8 recipe bytes.
The service does not canonicalize recipe JSON.
The manifest also records each output's length, hash, and facts checked from its encoded bytes.
Outputs contain no EXIF, GPS, XMP, ICC, or thumbnail metadata.
Different builds or pipeline versions can produce different encoded bytes.

## Image and color policy

The service accepts JPEG, PNG/APNG, GIF, WebP, AVIF, HEIC/HEIF, BMP, TIFF, JPEG XL, SVG, ICO, QOI, and PNM.
It detects each format from its bytes. HEIC/HEIF support uses the HEVC still-image codec.
PNG and APNG support valid depths through 16 bits, including palette and grayscale expansion.
TIFF supports one page. ICO selects the largest supported icon representation.
GIF, APNG, WebP, AVIF, and JPEG XL support animation.

The worker applies EXIF orientation and supported RGB ICC profiles before removing metadata.
It converts supported CICP/NCLX color primaries and transfer functions to sRGB.
CMYK JPEG uses its ICC profile when present. Adobe CMYK samples are inverted before the ICC transform.
Without a CMYK profile, the JPEG decoder uses its normal CMYK-to-RGB conversion.
The worker converts SDR 16-bit samples before quantizing them to 8-bit WebP pixels.
An ICC profile can be at most 1 MiB. Its declared length must match its actual length.
PNG metadata decoding has a separate 2 MiB allocation limit.
The worker treats ordinary unprofiled RGB images as sRGB.
It rejects malformed profiles, unsupported custom PNG gamma/chromaticity, explicit PQ/HLG HDR, and floating-point TIFF.
Resize uses an area filter in linear light with premultiplied alpha.
Lossless output preserves the normalized and resized pixels. It does not reconstruct the source file.

SVG support uses a static renderer. It includes embedded static JPEG, PNG, GIF, and WebP images.
The embedded images use the same orientation and color normalization as top-level input.
The renderer uses installed system fonts. The runtime container includes DejaVu fonts.
It rejects scripts, SMIL/CSS animation, escaped CSS, foreign objects, external resource references, and XML stylesheet instructions.
It also rejects animated embedded images and nested embedded SVG. It never flattens them silently.
SVG rendering uses an 8-bit premultiplied surface. Very low alpha can reduce hidden RGB precision.

## Animation preservation

Set `animation_policy` to `preserve`. Other values are invalid recipes.
The service preserves every complete composited display, alpha, frame order, and total playback count.
It retains the final frame delay and identical adjacent frames. It never substitutes the first frame.
APNG supports both default images that participate in the animation and separate default poster images.
The worker decodes and validates a separate poster but does not include it in the animated output.
APNG applies SOURCE/OVER blending and NONE/BACKGROUND/PREVIOUS disposal at 16-bit precision.

GIF repeat count `n` becomes `n+1` total plays. No loop extension means one play; zero means infinite repetition.
APNG `num_plays`, existing WebP loop counts, and JPEG XL loop counts already describe total plays.
AVIF finite repetition count `n` becomes `n+1` total plays. An unspecified count means one play.
Every infinite loop becomes WebP loop count `0`. Values above the WebP field range return an error.

GIF and WebP retain their integer millisecond delays.
APNG, AVIF, and JPEG XL round cumulative rational time to the nearest millisecond.
Each output delay is the difference between two rounded timestamps. This bounds cumulative rounding drift.
A zero or unrepresentable output delay returns `unrepresentable_animation`; no frame is dropped or merged.
`duration_ms` describes one complete cycle without repeats.
An animated container with one display frame retains its animation flag, duration, and loops.

HEIF timed tracks, multipage TIFF, and associated-alpha JPEG XL are not supported in this version.
HEIF timed tracks and unsupported SVG animation return errors instead of static output.
Multipage TIFF returns `unsupported_format`; associated-alpha JPEG XL returns `unsupported_color_profile`.
These exclusions require dedicated sequence, page-selection, or original-color unassociation support.

Static images have `animated=false`, `frame_count=1`, `duration_ms=0`, and `loop_count=1`.
Source dimensions for JPEG/PNG/GIF/WebP describe pixels before EXIF orientation.
HEIC, AVIF, and JPEG XL source dimensions describe display geometry after container transforms.
Output dimensions always describe the corrected pixels.

## Resource budgets

Operators can raise or lower the defaults below.
Schemas describe the protocol and codec constraints. Request validation uses the active operator settings.
See [operator budgets](../operations.md) for all settings and a larger example.

| Budget | Default | Scope |
| --- | --- | --- |
| Source bytes | 32 MiB | One request |
| Full request bytes | 33 MiB | Multipart envelope and both parts |
| Recipe bytes | 16 KiB | One recipe |
| Input side length | 8192 pixels | Each source side |
| Canvas pixels | 40 million | One source canvas |
| Frames | 120 | Full source |
| Total frame pixels | 100 million | Sum across source frames |
| Decoded work bytes | 256 MiB | Cumulative RGBA work across source frames |
| Working pixel budget | 256 MiB | Conservative live-pixel plan |
| Cycle duration | 60 seconds | One animation cycle |
| Encoded output bytes | 32 MiB | Sum across outputs |
| Outputs | 4 | One recipe |
| Admitted requests | 1 | Upload through response delivery |
| Absolute deadline | 60 seconds | One request |
| Input/output idle time | 10 seconds | Time without progress |
| Child address space | 768 MiB | Linux worker process |
| Child CPU time | 60 CPU seconds | Linux worker process |

All limits apply together.
The base working plan reserves 20 bytes per canvas pixel. APNG reserves 48 bytes per pixel.
HEIC also checks 20 bytes per coded tile/alpha pixel, before crop.
These checks can reject an image before its display canvas limit does.
The decoded-byte limit also applies before the 100-million-pixel limit at these defaults.
Native allocations have a separate Linux address-space and CPU limit.
The pixel plan does not measure total library allocations.
Decoded work bytes limit cumulative processing work. They do not mean that all raw frames occupy RAM.
Compressed muxes for all requested sizes remain in memory until the source finishes.
`HIRUNE_SPOOL_BYTES` and the `spool_bytes` field were removed in version 0.3.0.
`HIRUNE_CHILD_CPU_SECONDS` sets CPU time independently of the wall deadline.

`GET /v1/capabilities` reports active settings and fixed codec limits.
The independent response verifier accepts a capabilities snapshot with `--limits`.
Without that option, it checks the protocol and codec constraints without assuming default operator budgets.
Windows supports functional and process tests. Verify native memory enforcement on Linux.

## Admission, deadlines, and cleanup

The service has no admission waiting queue.
When all permits are in use, it returns `503` with `Retry-After: 1` before collecting a large body.
The absolute deadline covers upload, file operations, child execution, verification, and delivery on the server.

After the supervisor submits a file operation, it retains the permit until that operation and cleanup finish.
The HTTP timeout error can reach the caller before the permit returns.
Before response headers, a timeout returns `504` JSON.
After headers, a timeout or shutdown aborts the TCP connection and discards pending kernel writes before releasing the permit.
Callers must reject an incomplete multipart response.
Normal response completion uses graceful socket closure.

Cancellation can take time to stop native work.
The supervisor kills the child and waits for its exit before admitting replacement work.
The service removes request directories after success, failure, timeout, disconnect, and shutdown.

## Errors

Errors use the [fixed envelope](../../contracts/error-v1.schema.json).
The `error` object contains a code, a fixed English message, and `request_id`.
It contains no native diagnostics, filenames, source metadata, or image bytes.

| HTTP status | Error codes |
| --- | --- |
| 400 | `invalid_request`, `invalid_recipe` |
| 413 | `request_too_large`, `image_limits_exceeded`, `output_limits_exceeded` |
| 415 | `unsupported_format` |
| 422 | `invalid_image`, `unsupported_color_profile`, `unrepresentable_animation` |
| 503 | `processor_overloaded`, `processor_unavailable` |
| 504 | `processing_timeout` |
| 500 | `internal_error` |

## Health and discovery

`GET /health/live` and `GET /health/ready` return `204` when healthy.
Readiness becomes false during shutdown. A temporary lack of permits does not change readiness.
Startup probes the actual native codecs.
`GET /v1/capabilities` returns effective limits and component versions.
`GET /openapi.json` serves the generated [OpenAPI document](../openapi.json).
The [manifest schema](../../contracts/image-manifest-v1.schema.json) defines successful JSON facts.
