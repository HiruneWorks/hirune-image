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
It decodes the source pixels once, normalizes the frames, and writes RGBA frame files within the spool budget.
It reads those files to encode each output in sequence.
It does not need to hold every raw frame or run several encoders in memory at once.
The frame files are temporary data for that request. They are not durable jobs or external storage.
When the directory uses tmpfs, these files consume container memory.

The native writer enforces the output byte limit for each WebP frame.
The encoder sets the method, compression mode, and quality explicitly.
Native config uses `exact=1` and `alpha_quality=100`. Codec threads are disabled.
The animation mux uses complete canvas frames with no blending. It retains duplicate frames and their timing.
The worker checks the output structure and fully decodes each output before the API sends success headers.

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
