# Deployment

Build with docker compose -f deploy/compose.yml build. Run on a private internal network with the authorized caller attached; the example publishes no host port and grants no outbound route. No object storage, database or user credentials are needed.

The initial profile is non-root, read-only, dropped capabilities, no-new-privileges, 1GiB container memory, two CPUs, 64 PIDs and a 512MiB /tmp tmpfs. tmpfs is charged to container memory. Native children use a separate address-space/CPU fence. Raising container memory does not raise service limits.

Use /health/live for restart health, /health/ready for readiness, and /v1/capabilities to verify the exact policy/build/native versions. Wire network access control and caller retries at deployment. Authentication, durable jobs and publication remain caller responsibilities. Local fixture acceptance does not establish production capacity.
