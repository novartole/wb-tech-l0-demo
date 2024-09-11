# Build
FROM rust:1.81.0-alpine as builder

RUN --mount=type=cache,target=/var/cache/apk \
  apk update \
  && apk add musl-dev \
  && rustup target add aarch64-unknown-linux-musl

WORKDIR /build

COPY Cargo.toml Cargo.lock ./

RUN --mount=type=cache,target=/build/target \
  mkdir src \
  && echo "fn main() {}" > src/main.rs \
  && cargo build --release --target aarch64-unknown-linux-musl

COPY src/ src/

RUN --mount=type=cache,target=/build/target \
  touch src/main.rs \
  && cargo build --release --target aarch64-unknown-linux-musl \
  && mkdir /output \
  && cp target/aarch64-unknown-linux-musl/release/l_0_demo* /output/



# Run
FROM alpine as runtime

WORKDIR /opt/l_0_demo

COPY --from=builder output/l_0_demo* .

ENTRYPOINT ["./l_0_demo" ]
