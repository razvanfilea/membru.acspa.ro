ARG TARGET_ARCH=x86_64-unknown-linux-musl

FROM rust:1.86-alpine AS base
ARG TARGET_ARCH
USER root

RUN rustup target add $TARGET_ARCH && \
    apk add --no-cache musl-dev npm sccache && \
    cargo install cargo-chef

ENV RUSTC_WRAPPER=sccache SCCACHE_DIR=/sccache


FROM base AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base AS builder
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo chef cook --release --target $TARGET_ARCH --recipe-path recipe.json
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo build --release --target $TARGET_ARCH
RUN npm install && npm run prod


FROM scratch
ARG TARGET_ARCH

COPY --from=builder /app/assets/ ./assets/
COPY --from=builder /app/target/${TARGET_ARCH}/release/acspa ./

ENTRYPOINT ["./acspa"]
