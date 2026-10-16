# Architecture

Hirune Image is a stateless synchronous service. The root crate owns configuration, bootstrap, router construction, request supervision, and executable lifecycle. The application crate owns HTTP-free recipes, facts, validation, planning, budgets, and typed failures. The image-codec crate owns decode/color/resize/encode/verification and all unsafe bindings. Internal crates are source boundaries, not independently published packages.

An accepted request owns one admission permit from before body collection through completion or cancellation. Bounded multipart parsing produces source bytes and a typed recipe. A supervisor owns the request directory, permit, local file writes, and child. HTTP cancellation signals the supervisor; the supervisor retains resources until outstanding file I/O finishes and the child is killed/reaped.

The child validates structural headers before pixel allocation, decodes the source once, normalizes frames, and writes bounded RGBA frame files in its request directory. It then reads these files for each variant sequentially. This avoids holding every raw frame or four native encoders in memory. Spool files are local scratch data, not durable jobs or external storage. tmpfs consumes container memory.

Each WebP frame uses an output-bounded native writer with explicit method, lossless/quality, exact alpha, and disabled codec threads. Animated outputs use full-canvas, no-blend frames in a low-level mux, retaining duplicate frames and their timing. Output bytes are structurally inspected and fully decoded in the isolated child before any success headers.

The parent verifies the result binding, output order, geometry, lengths, hashes, and aggregate budgets. Delivery owns the permit and directory. Its watchdog can close the underlying TCP socket independently of body polling, discard buffered output, clean up, and release admission. Chunks handed to Hyper have independent bounded allocations. A body EOF is a server-side completion boundary, not acknowledgement that the caller consumed all bytes.

Health endpoints bypass image admission. Shutdown stops readiness, closes active transport, signals supervisors, and drains permits after child cleanup. Deployment owns private network access and container limits. No caller URL/path, credentials, durable queue, or storage authority is accepted.
