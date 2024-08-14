ARG TARGET_ARCH=x86_64-unknown-linux-musl

FROM rust:1.80-bookworm AS builder

RUN apt-get update && \
    apt-get install -y \
    musl-tools npm

ARG TARGET_ARCH

RUN rustup target add $TARGET_ARCH

# create a new empty shell project
RUN USER=root cargo new --bin acspa
WORKDIR /acspa

# copy manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# cache dependencies
RUN cargo build --release --target ${TARGET_ARCH}
RUN rm src/*.rs

# copy everything else
COPY . .

RUN npm i && npm run prod

RUN rm ./target/${TARGET_ARCH}/release/deps/acspa*
RUN cargo build --release --target ${TARGET_ARCH}

FROM scratch

ARG TARGET_ARCH

COPY --from=builder /acspa/assets/ ./assets/
COPY --from=builder /acspa/target/${TARGET_ARCH}/release/acspa ./

ENTRYPOINT ["./acspa"]
