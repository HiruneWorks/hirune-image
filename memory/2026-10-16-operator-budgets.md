# Operator-defined budgets and image regression coverage

Operators can raise or lower image, request, output, pixel, spool, native-memory, frame, duration, concurrency, and deadline budgets.
Defaults no longer act as fixed maximums.
Capabilities reports active budgets and fixed codec limits separately. CPU time has its own setting.
Codec limits, numeric representation limits, and checked arithmetic still apply.
The worker validates its CLI limits and applies them before reading job metadata derived from the request.
Both IPC directions use checked size limits that grow with the recipe and output count.

Astra reviewed configuration, semaphore representation, native conversions, IPC limits, and tests.
The reviewer found that both memory allocation outcomes returned the same HTTP failure.
The corrected helper returns success if allocation escapes the address-space limit. That success makes the test fail.
Tests check that the Linux PID is absent after the permit returns. They also check that the next request succeeds.

Local Windows workspace tests, clippy, and schema drift checks passed.
HTTP tests with larger budgets processed 8192-pixel output, 121 frames over 121 seconds, and 1024 maximum-length IDs.
The metadata exceeded the earlier fixed IPC limits. Schemas and an independent Python verifier checked the results.
Coverage includes independent LittleCMS RGB and alpha references, scalar resized-animation references, disposal, and every frame delay.
It also covers identical frames with finite loops, corrupt-tail rejection in first-frame mode, extreme geometry, and bounded header mutations.

Local Linux Docker tests verified address-space allocation denial and CPU termination.
They checked child exit, cleanup, recovery, operator budgets, and process cancellation.
Windows does not run the Linux resource tests.
These checks establish functional boundaries, not production capacity.
The benchmark follow-up milestone records container profiles and remote CI evidence.
