# Agent instructions

- Work autonomously within the requested scope; preserve unrelated changes.
- Read [conventions](docs/conventions.md), [requirements](docs/requirements.md), and the relevant [behavior contract](docs/behavior/README.md) before changing behavior.
- Use explicit names, focused files, declarative module indexes, thin handlers, and HTTP-free application services. Closed sets use enums. Validate HTTP and application boundaries.
- This is a synchronous image microservice. No databases, Cedar, sessions, object storage, persistent queues, or video without a new requirement.
- Confine native unsafe code to crates/image-codec; document ownership and check sizes before allocation. Supervise children through kill/reap/cleanup before admission release.
- Preserve .envs/.example/, compose/, deploy/, just/xtask, tracing, schema drift checks, and real HTTP/process tests.
- Never log image bytes, recipes, original filenames, source metadata, arbitrary child stderr, or credentials.
- Keep code/contracts/configuration/docs/tests aligned. Distinguish local, container, remote CI, deployment, and caller integration evidence.
- Use maintained Action tags. Make coherent local English commits and descriptive branches without an agent prefix. No push/publication without authorization.
- Record milestones in memory/YYYY-MM-DD-<scope>.md and its index; never store secrets.
