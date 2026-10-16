# Independent Sharp comparison and verification

The user chose an independent Sharp workload because the old Smol repository will be removed.
`benchmarks/sharp` contains a Node wrapper with locked Sharp 0.35.5. It needs no Smol checkout, image, or runtime.
The wrapper decodes source pixels once into oriented sRGB RGBA.
It resizes and encodes outputs in sequence, fully decodes each output, hashes bindings, and emits the benchmark wire format.
The production image excludes this trusted-fixture benchmark code.
The wrapper does not implement the service's full policy for hostile input or OS isolation.

Astra found that floating-point scaling and floor could turn 22→15 into 14.
Integer geometry and a regression test corrected that case.
The tool checks source, recipe, and output hashes, actual WebP structure, and independently expected dimensions.
It also checks cycle duration, loops, and exact Hirune frame counts.
Validation occurs after batch timing. Failed responses do not count as success.
CPU accumulates in a new cgroup for each scenario. `memory.peak` includes tmpfs and startup.
The run used shared developer hardware, different native versions, and different resize filters.
Those conditions do not support a universal performance or quality claim.

The results in `benchmarks/results/2026-10-16` contain 480 measured requests and 72 warmups. All succeeded.
Sharp was faster on the opaque lossy PNG with one output. Four opaque outputs were close.
Hirune was faster on these alpha, animation, ICC, and lossless fixtures, with lower memory peaks in this run.
This synthetic local comparison does not establish production acceptance.
Five short Windows Criterion measurements separately checked the component benchmark configuration.

Windows workspace checks passed formatting, clippy, schemas, and 31 tests. The later fixture-integrity test also passed.
Linux `cargo xtask check` passed 33 tests. The Sharp Docker build passed three tests.
The default and larger profiles each passed eight actual HTTP cases under production restrictions.
Operators can configure both larger application budgets and container settings.
CI checks both profiles before any version-tag publication.
This work created no release tag and performed no registry publication or deployment.
