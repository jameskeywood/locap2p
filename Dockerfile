# syntax=docker/dockerfile:1

FROM rust:1.99-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --locked --release -p locap2p-bootstrap -p locap2p-relay \
    && mkdir -p /out \
    && cp target/release/locap2p-bootstrap target/release/locap2p-relay /out/

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /out/ /usr/local/bin/

WORKDIR /app

EXPOSE 4001 4002

LABEL org.opencontainers.image.title="locap2p services"
LABEL org.opencontainers.image.description="Bootstrap and relay services for locap2p"
