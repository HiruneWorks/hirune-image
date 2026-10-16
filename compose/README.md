# Container configuration

The production image uses `production/app/Dockerfile`.
The [deployment configuration](../deploy/README.md) pulls the versioned GHCR image.
For local development, `compose.local.yml` adds a source build.
Run this command from the repository root:

```sh
docker compose -f deploy/compose.yml -f compose/compose.local.yml up --build -d
```

Run `just test-container` for actual HTTP checks against the image with production restrictions.
These include a non-root user, a read-only root filesystem, and resource limits.
CI sets `HIRUNE_TEST_IMAGE` to its candidate image and `HIRUNE_SKIP_BUILD=1` to reuse that image.
It sets `HIRUNE_EXPECT_BUILD_ID` to check the image's Git build identity.
