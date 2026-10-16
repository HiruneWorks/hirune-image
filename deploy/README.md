# Deployment

This is a synchronous Docker microservice. The caller sends source bytes and a recipe over HTTP; Hirune Image returns verified WebP parts. The caller owns authentication, persistent storage, retries and publication. No database or object storage is required inside this service.

After the `v0.2.0` release workflow succeeds, deploy its versioned GHCR image:

```sh
docker compose -f deploy/compose.yml pull
docker compose -f deploy/compose.yml up -d
```

The default image is `ghcr.io/hiruneworks/hirune-image:0.2.0`. Set `HIRUNE_IMAGE` to another published version or, preferably, the exact `ghcr.io/hiruneworks/hirune-image@sha256:...` reference recorded in the release workflow summary. Package visibility follows GHCR settings; a private package requires `docker login ghcr.io` with a credential authorized to read that package before pulling. Registry credentials are used by Docker, not passed to the service.

The example publishes no host port and grants no outbound route. Attach the authorized caller to the Compose `processing` network and use `http://image:8080` as its internal service URL. Configure network access at deployment because the service does not authenticate requests.

The initial profile is non-root, read-only, dropped capabilities, no-new-privileges, 1GiB container memory, two CPUs, 64 PIDs and a 512MiB /tmp tmpfs. tmpfs is charged to container memory. Native children use a separate address-space/CPU fence. Raising container memory does not raise service limits.

Use /health/live for restart health, /health/ready for readiness, and /v1/capabilities to verify the exact policy/build/native versions. Wire network access control and caller retries at deployment. Authentication, durable jobs and publication remain caller responsibilities. Local fixture acceptance does not establish production capacity.

## Release publication

The CI workflow runs code checks, builds one `linux/amd64` candidate with the Git commit build ID and OCI source/revision/version labels, and tests that candidate under the same non-root, read-only resource profile. It transfers the tested image as an artifact. The publication job checks the loaded image ID, then tags and pushes that same image to GHCR without rebuilding. Only this job receives `packages: write`; it authenticates using the workflow's automatic `GITHUB_TOKEN`. No additional publishing secret is needed. This follows [GitHub's GHCR authentication model](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry) and [Docker's image transfer workflow](https://docs.docker.com/build/ci/github-actions/share-image-jobs/).

For version `0.2.0`, tag the intended `main` commit after the workflow change has landed:

```sh
git switch main
git pull --ff-only
git tag -a v0.2.0 -m "Release 0.2.0"
git push origin v0.2.0
```

The pushed tag must exactly equal `v` plus `workspace.package.version` from `Cargo.toml`. Versions must also be valid Docker tags; Cargo build metadata containing `+` is rejected. A `v0.2.0` Git tag produces the image tag `:0.2.0`. There is no `latest` alias and no ARM image claim. Branch pushes, pull requests and `workflow_dispatch` perform verification without publication.

Version tags must not be moved or reused for changed code. GHCR version tags remain mutable registry names, so deploy by the recorded digest when exact artifact identity is required. Before the next release, update the workspace version, lockfile, this profile's default image tag and changelog together. Image publication does not deploy or restart any running service.

## Local container development

Build and run the current checkout with the local override:

```sh
docker compose -f deploy/compose.yml -f compose/compose.local.yml up --build -d
```

Run `just test-container` for local acceptance. The production profile itself pulls a published image; only the local override contains source-build configuration.

All application budgets and container resources are configurable. See [operator budgets](../docs/operations.md) and the [larger profile](../.envs/.example/large.env). Pass `--env-file .envs/.example/large.env` before `-f` to apply that Compose profile; confirm with `docker compose ... config` and the running capabilities endpoint.
