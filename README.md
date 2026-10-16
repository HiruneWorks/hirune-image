# Hirune Image

Hirune Image is a Rust/Axum image microservice with configurable resource limits.
Send JPEG, PNG, GIF, or WebP bytes with a typed recipe. The service returns verified lossy or lossless WebP outputs.
Run a published GHCR image with the [deployment Compose profile](deploy/README.md).
For development, build the service from source.

From the repository root, run `just build` to build the API and worker.
Run `just api` to start the service. Read `GET /v1/capabilities` to inspect its settings.
The service needs no database or storage credentials.

```sh
curl --fail-with-body http://127.0.0.1:8080/v1/images/process \
  -F 'file=@input.gif' \
  -F 'recipe=@contracts/examples/image-recipe-v1.json;type=application/json' \
  -o result.multipart
```

The success response uses `multipart/related`.
A JSON manifest comes first. Binary outputs follow in recipe order.
Before accepting a result, check source and recipe SHA-256, part identities, lengths, and output hashes.

Run `python3 scripts/verify_response.py result.multipart --source input.gif --recipe contracts/examples/image-recipe-v1.json` to check hashes, parts, and WebP structure independently.
Use `python` on Windows. The isolated worker also fully decodes each output before reporting success.

- [HTTP, image, budget and cancellation contract](docs/behavior/image-processing.md)
- [Recipe schema](contracts/image-recipe-v1.schema.json), [manifest schema](contracts/image-manifest-v1.schema.json), [error schema](contracts/error-v1.schema.json)
- [OpenAPI](docs/openapi.json), [architecture](docs/architecture.md), [conventions](docs/conventions.md)
- [Development and E2E](docs/development.md), [deployment](deploy/README.md), [verification milestones](memory/README.md)
- [Technical writing](docs/writing.md)

Run `just check` to check formatting, clippy, tests, and generated contracts.
Run `just test-container` to verify the Linux image with a non-root user and read-only root filesystem.
Each request has an isolated native worker.
Temporary files have byte limits. tmpfs consumes deployment memory.

Pushing `v0.2.0` publishes `ghcr.io/hiruneworks/hirune-image:0.2.0` after code and container checks pass.
The tag must match the workspace package version. Publication uses the exact tested `linux/amd64` image.
Branch pushes, pull requests, and manual checks do not publish images.
See the [release procedure](deploy/README.md#release-publication).

Operators can raise or lower budgets. See [configuration and memory](docs/operations.md) and the [larger example](.envs/.example/large.env).
[Criterion and Sharp service benchmarks](docs/benchmarks.md) measure pipeline components and complete HTTP requests separately.

The [recorded local comparison](benchmarks/results/2026-10-16/README.md) includes raw reports, versions, resource settings, and measurement limits.

Apache-2.0. Copyright 2026 Levi Laine.
