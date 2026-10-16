# Environment settings

Examples live in .example/app.env and .example/large.env. Copy to ignored .local/app.env and use just, or inject values explicitly in the deployment. Operators may raise or lower budgets within codec/representation limits; invalid combinations fail startup. Active limits and fixed codec limits are returned separately by /v1/capabilities. See [operator budgets](../docs/operations.md) for memory, tmpfs and container sizing. No credentials are required.

Variables: HIRUNE_BIND, HIRUNE_WORKER (operator-owned executable path), HIRUNE_TEMP_ROOT, RUST_LOG; HIRUNE_SOURCE_BYTES, HIRUNE_REQUEST_BYTES, HIRUNE_RECIPE_BYTES, HIRUNE_INPUT_DIMENSION, HIRUNE_CANVAS_PIXELS, HIRUNE_FRAMES, HIRUNE_TOTAL_FRAME_PIXELS, HIRUNE_DECODED_BYTES, HIRUNE_WORKING_BYTES, HIRUNE_SPOOL_BYTES, HIRUNE_DURATION_MS, HIRUNE_OUTPUTS, HIRUNE_OUTPUT_BYTES, HIRUNE_OUTPUT_DIMENSION, HIRUNE_CONCURRENCY, HIRUNE_DEADLINE_MS, HIRUNE_IDLE_MS, HIRUNE_CHILD_MEMORY_BYTES.

HIRUNE_CHILD_CPU_SECONDS sets consumed child CPU seconds independently of the absolute wall deadline. HIRUNE_CONTAINER_MEMORY, HIRUNE_TMPFS_SIZE, HIRUNE_CONTAINER_CPUS and HIRUNE_CONTAINER_PIDS configure the Compose container.

HIRUNE_BUILD_ID is injected at compile time, not read from a caller or runtime request. An ordinary local build identifies itself as development.
