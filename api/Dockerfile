# syntax=docker/dockerfile:1.7

FROM rust:1.92-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY db ./db
COPY src ./src
RUN --mount=type=cache,id=orchard-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=orchard-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=orchard-api-target-rust-1.92,target=/build/target,sharing=locked \
    cargo build --locked --release --bin orchard --jobs 1 \
    && install --mode=0755 /build/target/release/orchard /build/orchard

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/orchard /usr/local/bin/orchard

USER nobody
EXPOSE 3000
ENTRYPOINT ["orchard"]
