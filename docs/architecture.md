# Architecture

Hirune Image is a stateless service. Each processing request returns its outputs synchronously.
The root crate owns configuration, startup, routes, request supervision, and shutdown.
The application crate owns recipes, facts, validation, planning, budgets, and typed errors. It has no HTTP dependency.
The image-codec crate owns decoding, color conversion, resizing, encoding, verification, and all unsafe bindings.
The crates separate source responsibilities. They are not published as independent packages.

Before collecting the body, an accepted request acquires one admission permit.
The permit reserves capacity until the response completes or cancellation cleanup finishes.
Multipart parsing enforces byte limits and produces source bytes and a typed recipe.
The supervisor owns the request directory, permit, file writes, and child process.
HTTP cancellation signals the supervisor.
The supervisor waits for pending file operations to finish and for the stopped child to exit before it releases resources.

The worker checks image headers before allocating pixel buffers.
It decodes and normalizes each display frame, then resizes and encodes that frame for every requested output.
It runs the output encoders in sequence. Each output keeps its compressed chunks in a separate bounded mux.
The worker releases the display pixels before processing the next frame. It writes no raw RGBA frame files.
Accurate decoding remains the default. Optional JPEG preview uses a shared reduced-IDCT canvas when every output fits it.
The callback carries original display geometry and the source working reservation separately from the decoded pixel dimensions.
Source facts and source budgets always describe the original image.
The JPEG preview decoder uses `jpeg-decoder` 0.3.2 without Rayon. ICC and CMYK input retain the accurate decoder.
Decoder canvases, reference frames, parsing data, and working buffers still use memory.
JPEG XL uses an allocation tracker. The worker clears its display render cache between frames.
HEIC checks every SPS's coded size before conformance cropping, including grid tiles and alpha items.
PNG container inspection checks chunk bounds and significant CRCs before pixel decoding.
PNG color selection resolves declaration priority and builds one in-memory SDR profile for static or animated frames.
The worker uses the existing CMS transform before it quantizes 16-bit pixels. It creates no intermediate ICC file.

The native WebP writer enforces the remaining output byte limit for each encoded frame.
The Rust mux checks the combined output byte budget before adding each frame.
It writes complete canvas frames with no blending. It retains duplicate frames and one-frame animations.
The mux builds the final RIFF buffer directly. It does not assemble a second native copy of the animation.
Native encoder config uses `exact=1` and `alpha_quality=100`. Codec threads are disabled.
The resizer caches only axis geometry. It does not allocate a floating-point source canvas.
The cache planner checks all output plans together with the source pixel reservation before it allocates them.
It uses a conservative 48-byte-per-pixel reservation for PNG/APNG and the base 20-byte reservation for other accurate input.
JPEG preview also reserves full-resolution, MCU-padded coefficients and reduced pixel buffers.
If that reservation exceeds the working budget, it uses the accurate decoder before allocating coefficient storage.
If the cache does not fit, the worker uses the same span calculations without cache allocations.
An output with unchanged dimensions goes directly to the encoder without cloning its RGBA pixels.
The worker verifies each completed WebP before it writes the output file.
The API sends success only after every source frame and every output passes verification.
Request directories contain only the compressed source, encoded outputs, and control data.
When the directory uses tmpfs, those files also consume container memory.

The API process checks source and recipe bindings, output order, dimensions, lengths, hashes, and total output bytes.
Response delivery owns the permit and request directory.
Its watchdog can close the TCP socket even when Hyper stops polling the response body.
After it discards pending output, the watchdog cleans the directory and releases the permit.
Each image-data chunk passed to Hyper has its own allocation of at most 64 KiB.
The manifest is sent as one chunk. Its size depends on the output count and metadata budget.
End of body marks completion on the server. It does not prove that the caller received all bytes.

Health endpoints do not need an image admission permit.
During shutdown, the API stops readiness, closes active connections, and signals supervisors.
It waits for permits to return after child cleanup.
The deployment controls private network access and container limits.
The API accepts image bytes and recipes. It accepts no caller URL, file path, credentials, durable job, or storage authority.
