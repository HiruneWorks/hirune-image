# Deployment

Hirune Image is a synchronous Docker service.
The caller sends source bytes and a recipe over HTTP. The service returns verified WebP parts.
The caller owns authentication, persistent storage, retries, and publication.
The service needs no database or object storage.

## Run a published image

First, confirm that the `v0.2.0` release workflow succeeded.
Then run these commands from the repository root to pull and start its image:

```sh
docker compose -f deploy/compose.yml pull
docker compose -f deploy/compose.yml up -d
```

The default image is `ghcr.io/hiruneworks/hirune-image:0.2.0`.
Set `HIRUNE_IMAGE` to use another published version or the digest recorded in the release workflow summary.
For exact image identity, prefer `ghcr.io/hiruneworks/hirune-image@sha256:...`.

GHCR settings control package visibility.
Before pulling a private package, run `docker login ghcr.io` with a credential that can read the package.
Docker uses this credential. The service does not receive it.

## Network access and resource limits

The example exposes no host port and has no outbound network route.
Attach the authorized caller to the Compose `processing` network.
Use `http://image:8080` as the service URL within that network.
The service does not authenticate requests, so configure access control in the deployment.

The initial profile uses these settings:

| Setting | Value |
| --- | --- |
| User | Non-root |
| Root filesystem | Read-only |
| Linux capabilities | All dropped |
| New privileges | Disabled |
| Container memory | 1 GiB |
| CPU quota | 2 CPUs |
| Process count limit | 64 PIDs |
| `/tmp` tmpfs | 512 MiB |

tmpfs consumes container memory. Each child also has its own address-space and CPU limits.
Increasing container memory does not increase service budgets.

Use `/health/live` for process health and `/health/ready` for readiness.
Read `/v1/capabilities` to check the active policy, build identity, limits, and native versions.
Configure caller retries and network access control in the deployment.
Authentication, durable jobs, and publication remain the caller's responsibility.
Local fixture checks do not establish production capacity.

## Release publication

The CI workflow runs code checks, then builds one `linux/amd64` candidate.
The image contains the Git commit build ID and OCI source, revision, version, and license labels.
CI tests the candidate with the production user, filesystem, and resource restrictions.
It then transfers that tested image as an artifact.

The publication job checks the loaded image ID, tags the image, and pushes it without rebuilding.
Only that job receives `packages: write`.
It uses the workflow's automatic `GITHUB_TOKEN`; no extra publishing secret is required.
This follows [GitHub's GHCR authentication model](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry) and [Docker's image transfer workflow](https://docs.docker.com/build/ci/github-actions/share-image-jobs/).
The workflow summary records the published registry digest.

For version `0.2.0`, select the intended `main` commit after the workflow change has landed.
Run these commands from the repository root:

```sh
git switch main
git pull --ff-only
git tag -a v0.2.0 -m "Release 0.2.0"
git push origin v0.2.0
```

The tag must be exactly `v` followed by `workspace.package.version` from `Cargo.toml`.
The version must also be a valid Docker tag. Cargo build metadata with `+` is rejected.
The Git tag `v0.2.0` publishes the image tag `:0.2.0`.
The workflow does not publish a `latest` alias or an ARM image.
Branch pushes, pull requests, and `workflow_dispatch` run checks without publishing an image.

Do not move or reuse a release tag for changed code.
Registry version tags can still change, so deploy by digest when exact image identity matters.
Before the next release, update the workspace version, lockfile, default deployment image tag, and changelog together.
Publishing an image does not deploy it or restart a running service.

## Local container development

Use the local override to build and run the current checkout:

```sh
docker compose -f deploy/compose.yml -f compose/compose.local.yml up --build -d
```

Run `just test-container` to check the local image.
The production profile pulls a published image. The local override adds the source build configuration.

All application budgets and container resources are configurable.
See [operator budgets](../docs/operations.md) and the [larger profile](../.envs/.example/large.env).
Pass `--env-file .envs/.example/large.env` before `-f` to select that profile.
Check the result with `docker compose ... config`, then confirm active values through capabilities.
