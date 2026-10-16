# Container configuration

The production image is built from production/app/Dockerfile. Runtime wiring in [deploy](../deploy/README.md) pulls the versioned GHCR image. compose.local.yml overrides it with a source build for local development:

```sh
docker compose -f deploy/compose.yml -f compose/compose.local.yml up --build -d
```

Use `just test-container` for real HTTP checks against the non-root, read-only image with resource limits. CI passes an already built release candidate using `HIRUNE_TEST_IMAGE` and `HIRUNE_SKIP_BUILD=1`, and checks its Git build identity using `HIRUNE_EXPECT_BUILD_ID`.
