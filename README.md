# Hirune Image

A bounded Rust/Axum image microservice. JPEG, PNG, GIF and WebP bytes plus a typed recipe produce verified lossy/lossless WebP variants.

Build both executables with `just build`, run `just api`, and inspect `GET /v1/capabilities`. There are no database or storage credentials.

```sh
curl --fail-with-body http://127.0.0.1:8080/v1/images/process \
  -F 'file=@input.gif' \
  -F 'recipe=@contracts/examples/image-recipe-v1.json;type=application/json' \
  -o result.multipart
```

The success response is multipart/related: JSON manifest followed by binary outputs in recipe order. Validate source/recipe SHA-256, part identities, lengths and output hashes before accepting it.

Run `python3 scripts/verify_response.py result.multipart --source input.gif --recipe contracts/examples/image-recipe-v1.json` (`python` on Windows) for independent hash/part/WebP-structure checks. The isolated worker additionally performs full codec decoding before success.

- [HTTP, image, budget and cancellation contract](docs/behavior/image-processing.md)
- [Recipe schema](contracts/image-recipe-v1.schema.json), [manifest schema](contracts/image-manifest-v1.schema.json), [error schema](contracts/error-v1.schema.json)
- [OpenAPI](docs/openapi.json), [architecture](docs/architecture.md), [conventions](docs/conventions.md)
- [Development and E2E](docs/development.md), [deployment](deploy/README.md), [verification milestones](memory/README.md)

Run `just check` for formatting, clippy, tests and contract drift. Run `just test-container` for the non-root, read-only Linux image acceptance. The native worker is isolated per request; bounded local scratch files and tmpfs count toward deployment resources.

Apache-2.0. Copyright 2026 Levi Laine.
