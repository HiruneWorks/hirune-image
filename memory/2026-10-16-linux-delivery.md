# 2026-10-16 — Linux delivery cancellation

## Changes

Split the unpublished implementation into service, test, container/CI and documentation commits after the full template import. Verified the resulting tree was identical to the prior local implementation before pushing the feature branch.

The first remote Linux CI run, 38855288098, passed image API, contract, limit and decode-once tests but failed the slow TCP consumer process regression. Reproduced its exact `transport remained open` failure in a local Rust 1.98.1 Linux container with `cargo test --locked --test process slow_tcp_consumer_is_closed_before_admission_is_reused -- --exact --nocapture`.

## Decisions and constraints

Default socket close can leave kernel-queued writes draining in the background. Cancellation now uses a separate `Connection::abort()` with zero linger to discard queued writes before releasing admission. Delivery timeout and server shutdown use abort. Normal `GuardedIo` drop retains graceful close so successful queued responses can drain. Socket configuration failures log only the fixed operation and error kind.

Astra reviewed the Linux close semantics and recommended separating abort from normal close. A follow-up review also identified completion racing with the delivery watchdog: recheck the finished token while holding the delivery mutex before aborting, so an already completed response or subsequent keep-alive request cannot be reset by a stale timer. Kept the original small-receive-buffer regression and its time limits unchanged.

## Validation

The same Linux regression failed before the fix in 8.49 seconds and passed after the fix in 5.16 seconds. Windows `just check` passed all 24 tests plus formatting, clippy and contract drift, including after the final watchdog completion guard. The complete Linux `cargo test --locked --workspace` suite also passed all 24 tests.

The minimal Rust Linux image provides `python3` without a `python` alias. Updated the independent-verifier test and the container acceptance recipe to choose `python` on Windows and `python3` on Unix; the Windows verifier regression passed again. Python 3 is now an explicit development prerequisite. Subsequent GitHub Actions runs track remote integration results. The earlier implementation milestone remains a record of verification before remote integration.

## Remaining work

Production deployment and capacity acceptance remain outside this change. No release tag or package/container publication is part of the Git integration.
