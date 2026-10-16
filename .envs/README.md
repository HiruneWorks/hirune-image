# Environment settings

Examples live in .example/app.env. Copy to ignored .local/app.env and use just, or inject values explicitly in the deployment. Every HIRUNE limit may only lower the reviewed ceiling; invalid combinations fail startup. Defaults and effective values are returned by /v1/capabilities. No credentials are required.

Variables: HIRUNE_BIND, HIRUNE_WORKER (operator-owned executable path), HIRUNE_TEMP_ROOT, RUST_LOG; HIRUNE_SOURCE_BYTES, HIRUNE_REQUEST_BYTES, HIRUNE_RECIPE_BYTES, HIRUNE_INPUT_DIMENSION, HIRUNE_CANVAS_PIXELS, HIRUNE_FRAMES, HIRUNE_TOTAL_FRAME_PIXELS, HIRUNE_DECODED_BYTES, HIRUNE_WORKING_BYTES, HIRUNE_SPOOL_BYTES, HIRUNE_DURATION_MS, HIRUNE_OUTPUTS, HIRUNE_OUTPUT_BYTES, HIRUNE_OUTPUT_DIMENSION, HIRUNE_CONCURRENCY, HIRUNE_DEADLINE_MS, HIRUNE_IDLE_MS, HIRUNE_CHILD_MEMORY_BYTES.

HIRUNE_BUILD_ID is injected at compile time, not read from a caller or runtime request. An ordinary local build identifies itself as development.
