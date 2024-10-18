ARG TARGET_ARCH=x86_64-unknown-linux-musl

FROM rust:1.82-alpine AS base
USER root

RUN apk add --no-cache deno musl-dev

ARG TARGET_ARCH
RUN rustup target add $TARGET_ARCH

RUN cargo install cargo-chef
WORKDIR /acspa


FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM base AS builder
COPY --from=planner /acspa/recipe.json recipe.json
RUN cargo chef cook --release --target $TARGET_ARCH --recipe-path recipe.json
COPY . .
RUN deno task prod
RUN cargo build --release --target $TARGET_ARCH


FROM scratch

ARG TARGET_ARCH

COPY --from=builder /acspa/assets/ ./assets/
COPY --from=builder /acspa/target/${TARGET_ARCH}/release/acspa ./

ENTRYPOINT ["./acspa"]
