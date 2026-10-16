# Agent instructions

## Work within the request

- Work autonomously within the requested scope. Preserve unrelated changes.
- Before changing behavior, read [conventions](docs/conventions.md), [requirements](docs/requirements.md), and the relevant [behavior contract](docs/behavior/README.md).
- This is a synchronous image microservice. Add no database, Cedar, sessions, object storage, persistent queue, or video without a new requirement.

## Keep responsibilities explicit

- Use explicit names, focused files, and module indexes that only declare or export modules.
- Keep HTTP handlers thin. Keep application services free of HTTP dependencies.
- Use enums for closed sets of values. Validate both HTTP and application inputs.
- Keep native unsafe code in `crates/image-codec`. Document ownership and check sizes before allocation.
- Retain the admission permit until submitted file operations finish, the child exits, and cleanup completes. On cancellation, kill the child and wait for its exit.
- Never log image bytes, recipes, original filenames, source metadata, arbitrary child stderr, or credentials.

## Write clear technical prose

**Write this 80% of the way to ASD-STE100.**

Apply this direction to specifications, runbooks, READMEs, technical documents, release notes, verification records, and code comments.
Follow [the writing guide](docs/writing.md). Treat 80% as a clarity target.
Preserve technical meaning, exact identifiers, commands, limits, and measured values.

## Check and record changes

- Preserve `.envs/.example/`, `compose/`, `deploy/`, `just`/`xtask`, tracing, schema drift checks, and actual HTTP/process tests.
- Update code, contracts, configuration, documents, and tests together.
- Report local tests, container tests, remote CI, deployment, and caller integration as separate results.
- Use maintained GitHub Action tags.
- Make coherent local English commits. Use descriptive branch names without an agent prefix.
- Push or publish only with authorization.
- Record milestones in `memory/YYYY-MM-DD-<scope>.md` and its index. Never store secrets.
