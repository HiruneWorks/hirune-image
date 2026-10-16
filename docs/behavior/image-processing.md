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

The service accepts JPEG, PNG, GIF, and WebP.
PNG supports valid depths up to 8 bits, including palette and grayscale expansion.
It rejects SVG, APNG, AVIF, HEIC, HDR, 16-bit PNG, and CMYK JPEG.

Before removing metadata, the worker applies EXIF orientation 1 through 8 to the pixels.
It converts supported embedded RGB ICC profiles to sRGB with moxcms.
An ICC profile can be at most 1 MiB. Its declared length must match its actual length.
PNG metadata decoding has a separate 2 MiB allocation limit.
The service rejects unsupported or malformed profiles.

The worker treats ordinary images without a profile as sRGB.
It rejects custom PNG gamma or chromaticity without a supported profile. It also rejects explicit HDR signaling.
Resize uses an area filter in linear light with premultiplied alpha.
Lossless output preserves the normalized and resized pixels. It does not reconstruct the original source file.

## Animation policy

Choose `preserve`, `first_frame`, or `reject`.
The `reject` policy rejects animated input.
The `preserve` policy keeps complete composited displays, alpha, every frame delay, and the total playback count.
This includes the final frame's delay.
The mux writes complete canvas frames and retains identical adjacent frames.

GIF loop values map to WebP as follows:

| GIF value | WebP total playback count |
| --- | --- |
| Finite repeat count `n` | `n + 1` |
| No loop extension | 1 |
| Infinite repeat | 0 |

The service rejects a loop count that overflows the WebP field. It retains existing WebP loop counts.
`duration_ms` describes one cycle, without repeats.
Animated input with a zero frame delay is rejected as unrepresentable.
An animated WebP container with one frame is also rejected.
The service preserves positive short delays numerically. Browsers can apply their own playback rules.

A static image has `frame_count=1`, `duration_ms=0`, and `loop_count=1`.
A one-frame GIF counts as a static image.
For `first_frame`, the worker still validates and budgets the full source. Only the output becomes static.
Source dimensions describe pixels before orientation. Output dimensions describe the corrected pixels.

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
| Decoded RGBA bytes | 256 MiB | Sum across source frames |
| RGBA spool bytes | 256 MiB | Temporary frame files for one request |
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
The working plan reserves 20 bytes per canvas pixel. It can reject an image before the canvas limit does.
The decoded-byte limit also applies before the 100-million-pixel limit at these defaults.
Native allocations have a separate Linux address-space and CPU limit.
The pixel plan does not measure total library allocations.
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
| 422 | `invalid_image`, `unsupported_color_profile`, `animation_not_allowed`, `unrepresentable_animation` |
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
