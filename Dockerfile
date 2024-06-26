# fetch the vendor with the builder platform to avoid qemu issues
FROM --platform=$BUILDPLATFORM rust:1-bookworm AS vendor

ENV USER=root

WORKDIR /code
RUN cargo init
COPY Cargo.toml /code/Cargo.toml
COPY Cargo.lock /code/Cargo.lock

# https://docs.docker.com/engine/reference/builder/#run---mounttypecache
RUN --mount=type=cache,target=$CARGO_HOME/git,sharing=locked \
    --mount=type=cache,target=$CARGO_HOME/registry,sharing=locked \
    mkdir -p /code/.cargo \
    && cargo vendor >> /code/.cargo/config.toml

FROM rust:1-bookworm AS builder

ENV USER=root

WORKDIR /code

COPY Cargo.toml /code/Cargo.toml
COPY Cargo.lock /code/Cargo.lock
COPY src /code/src
COPY --from=vendor /code/.cargo /code/.cargo
COPY --from=vendor /code/vendor /code/vendor

COPY src /code/src

RUN cargo build --release --offline

FROM scratch AS binary

COPY --from=builder /code/target/release/catapulte /catapulte

FROM debian:bookworm-slim

LABEL org.label-schema.url="https://github.com/jdrouet/tiny-vector"
LABEL maintaner="Jeremie Drouet <jeremie.drouet@gmail.com>"

COPY --from=builder /code/target/release/tiny-vector /usr/bin/tiny-vector

EXPOSE 3000

HEALTHCHECK --interval=10s --timeout=3s \
    CMD curl --fail --head http://localhost:3000/status || exit 1

ENTRYPOINT [ "/usr/bin/tiny-vector" ]
