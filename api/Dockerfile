FROM rust:1.92-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY db ./db
COPY src ./src
RUN cargo build --locked --release --bin orchard

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/orchard /usr/local/bin/orchard

USER nobody
EXPOSE 3000
ENTRYPOINT ["orchard"]
