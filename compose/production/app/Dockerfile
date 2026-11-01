FROM rust:1.99.0-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends cmake nasm pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY .cargo .cargo
COPY crates crates
COPY src src
COPY xtask xtask
COPY benches benches
COPY tests/support/resource_worker.rs tests/support/resource_worker.rs
ARG HIRUNE_BUILD_ID=development
ENV HIRUNE_BUILD_ID=$HIRUNE_BUILD_ID
RUN cargo build --locked --release --bin api --bin image_worker

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends fonts-dejavu-core libstdc++6 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/api /app/api
COPY --from=build /app/target/release/image_worker /app/image_worker
COPY crates/image-codec/native/avif-sys/licenses /usr/share/licenses/hirune-image/native-avif
USER 65532:65532
ENV HIRUNE_BIND=0.0.0.0:8080 HIRUNE_TEMP_ROOT=/tmp RUST_LOG=info
EXPOSE 8080
ENTRYPOINT ["/app/api"]
