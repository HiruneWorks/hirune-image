# 2026-11-02 — Linux container verification

## Changes

Resume Linux verification after Docker Desktop became available.
Check the image-processing implementation from commit `00e785cf555596094f8bfe59c9e0ad2d6a828ab4`.
Correct the container verifier's HTTP case count.
The report now counts successful processing requests instead of reporting the former fixed value of eight.
The current fixture list has eleven sources and two encoding modes, for 22 requests per profile.
This correction does not change the API or codec behavior.

## Decisions and constraints

Use the existing production Dockerfile and the committed synthetic fixtures.
Use an internal Docker network, a non-root service user, a read-only root filesystem, a temporary filesystem, dropped capabilities, and resource limits.
Test the default and large operator profiles against the same image.
These small functional fixtures do not establish production capacity.
Do not publish an image or deploy a service in this verification.

## Validation

The [Linux CI verification job](https://github.com/HiruneWorks/hirune-image/actions/runs/41318220141/job/124882697587) passed for implementation commit `00e785c`.
It ran formatting, Clippy with warnings denied, all workspace tests, and generated-contract drift checks.
The 14 format tests passed on Linux.
`actual_native_memory_and_cpu_fences_reap_and_recover` also passed.
That test checks actual address-space and CPU exhaustion, child exit, cleanup, and recovery for the next request.
The container verifier's Python syntax check passed locally.

The [CI container job](https://github.com/HiruneWorks/hirune-image/actions/runs/41318220141/job/124883511702) also passed both runtime profiles for `00e785c`.
The publication job was skipped because this was a branch push.

Local Docker Engine 28.3.3 built the production Linux image successfully.
The local tag is `hirune-image:streaming-variants-00e785c`.
The image ID is `sha256:0ed8606ade976bea143f83559ecb2a2cb83376dcd44f34a172aee5e327f915c3`.
Its reported build ID starts with the full implementation commit SHA.

Both local runtime profiles passed against that same image:

| Profile | Processing requests | WebP outputs checked | Container memory limit | Observed cgroup memory peak |
| --- | --- | --- | --- | --- |
| Default | 22 | 44 | 1,073,741,824 bytes | 6,688,768 bytes |
| Large | 22 | 44 | 4,294,967,296 bytes | 6,926,336 bytes |

The verifier checked every response with the independent response verifier.
It checked codec versions, build identity, active settings, non-root UID `65532`, an empty temporary directory after requests, and no container OOM kill.
The temporary test containers and networks were removed.
These peak values describe only the small committed fixtures. They are not per-request memory bounds or production capacity measurements.
The corrected report emitted `http_cases=22` for each profile.
Publication, deployment, and caller integration did not run.

## Remaining work

Measure production workloads before changing capacity settings.
