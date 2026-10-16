# Conventions

Use explicit descriptive names and direct implementations. Each file owns one responsibility. Module indexes only declare or re-export modules. Comments briefly explain invariants in English.

The root crate composes configuration, bootstrap, HTTP, process supervision, and lifecycle. Versioned handlers and exclusive DTOs live in src/http/v1/<feature>/; routes.rs owns URL/method composition. Application services remain unversioned and HTTP-free. Native bindings and unsafe ownership live in crates/image-codec.

Closed sets use enums. Unknown and duplicate fields are errors. DTO validation does not replace service validation. Public errors contain fixed codes/messages, never native diagnostics or submitted data.

Settings are explicit. .envs/ holds examples/ignored settings; compose/ holds image/local configuration; deploy/ holds runtime wiring. Use locked Cargo and just/xtask. Tests use the production router and actual child. Keep schemas, OpenAPI, examples, docs, and behavior aligned.

Preserve unrelated work. Make coherent local commits; no push/publication/deployment without authorization. Record verification and limitations in memory/.
