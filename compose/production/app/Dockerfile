FROM rust:1.98.1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY .cargo .cargo
COPY crates crates
COPY src src
COPY xtask xtask
ARG HIRUNE_BUILD_ID=development
ENV HIRUNE_BUILD_ID=$HIRUNE_BUILD_ID
RUN cargo build --locked --release --bin api --bin image_worker

FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=build /app/target/release/api /app/api
COPY --from=build /app/target/release/image_worker /app/image_worker
USER 65532:65532
ENV HIRUNE_BIND=0.0.0.0:8080 HIRUNE_TEMP_ROOT=/tmp RUST_LOG=info
EXPOSE 8080
ENTRYPOINT ["/app/api"]
