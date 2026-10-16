# 2026-10-16 — Linux delivery cancellation

## Changes

After the full template import, split the unpublished implementation into service, test, container/CI, and documentation commits.
Before pushing the branch, verified that its tree matched the previous local implementation.

Remote Linux CI run `38855288098` passed image API, contract, limit, and decode-once tests.
The slow TCP consumer regression failed.
Reproduced the exact `transport remained open` failure in a local Rust 1.98.1 Linux container with `cargo test --locked --test process slow_tcp_consumer_is_closed_before_admission_is_reused -- --exact --nocapture`.

## Decisions and constraints

Default socket close can leave queued kernel writes draining in the background.
Cancellation uses a separate `Connection::abort()` with zero linger.
It discards queued writes before releasing the admission permit. Delivery timeout and server shutdown use this abort path.
Normal `GuardedIo` drop keeps graceful close so successful queued responses can drain.
Socket configuration failures log only the fixed operation and error kind.

Astra recommended separate paths for abort and normal close on Linux.
A second review found a race between response completion and the delivery watchdog.
After acquiring the delivery mutex, the watchdog must check the finished token before aborting.
This prevents an old timer from resetting a completed response or a later keep-alive request.
The original regression kept its small receive buffer and time limits.

## Validation

The Linux regression failed in 8.49 seconds before the fix. It passed in 5.16 seconds after the fix.
Windows `just check` passed all 24 tests, formatting, clippy, and contract drift checks.
It also passed after the final watchdog completion guard.
Linux `cargo test --locked --workspace` passed all 24 tests.

The minimal Rust Linux image provides `python3` but no `python` alias.
Updated the verifier test and container check command to select `python` on Windows and `python3` on Unix.
The Windows verifier regression passed again. Python 3 became an explicit development prerequisite.
Later GitHub Actions runs track remote integration results.
The earlier implementation milestone records the checks performed before remote integration.

## Status at this milestone

This change did not include production deployment or capacity acceptance.
The Git integration did not create a release tag or publish a package or container.
