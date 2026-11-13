FROM --platform=$BUILDPLATFORM rust:1.99.1-bookworm AS build
ARG TARGETARCH
RUN apt-get update && apt-get install -y --no-install-recommends cmake nasm pkg-config gcc-aarch64-linux-gnu g++-aarch64-linux-gnu && rm -rf /var/lib/apt/lists/*
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
    CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++ \
    AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar \
    CARGO_BUILD_JOBS=2 \
    HIRUNE_CROSS_ARCH=$TARGETARCH \
    CMAKE=/usr/local/bin/cmake-target
COPY compose/production/app/cmake-target.sh /usr/local/bin/cmake-target
COPY compose/production/app/arm64.cmake /opt/toolchains/arm64.cmake
RUN chmod 755 /usr/local/bin/cmake-target
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
RUN case "$TARGETARCH" in amd64) target=x86_64-unknown-linux-gnu ;; arm64) target=aarch64-unknown-linux-gnu ;; *) exit 1 ;; esac \
    && rustup target add "$target" \
    && cargo build --locked --release --target "$target" --bin api --bin image_worker \
    && install -d /artifacts \
    && install "target/$target/release/api" "target/$target/release/image_worker" /artifacts/

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends fonts-dejavu-core libstdc++6 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /artifacts/api /app/api
COPY --from=build /artifacts/image_worker /app/image_worker
COPY crates/image-codec/native/avif-sys/licenses /usr/share/licenses/hirune-image/native-avif
USER 65532:65532
ENV HIRUNE_BIND=0.0.0.0:8080 HIRUNE_TEMP_ROOT=/tmp RUST_LOG=info
EXPOSE 8080
ENTRYPOINT ["/app/api"]
