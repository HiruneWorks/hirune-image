# Conventions

Use explicit names and direct implementations. Give each file one responsibility.
Module index files declare modules or export names. Code comments explain invariants in English.

For technical documents and comments: **Write this 80% of the way to ASD-STE100.**
Follow [the writing guide](writing.md). Use short sentences, active verbs, and consistent terms.
Keep identifiers, conditions, commands, and measured values exact.

The root crate connects configuration, startup, HTTP, process supervision, and shutdown.
Put versioned handlers and their DTOs in `src/http/v1/<feature>/`.
Declare URL and method mappings in `routes.rs`.
Application services have no HTTP dependency or API version.
Put native bindings and unsafe ownership code in `crates/image-codec`.

Use enums for closed sets of values. Reject unknown and duplicate fields.
Validate both DTOs and application service inputs.
Public errors contain fixed codes and messages. Keep native diagnostics and submitted data out of public errors.

Declare settings explicitly. Put examples and ignored local settings in `.envs/`.
Put image build and local container configuration in `compose/`. Put deployment configuration in `deploy/`.
Use locked Cargo dependencies and the `just` or `xtask` commands.
Tests use the production router and an actual child process.
Update schemas, OpenAPI, examples, documents, and behavior together.

Preserve unrelated work. Make coherent local commits.
Push, publish, or deploy only with authorization. Record checks and their limits in `memory/`.
