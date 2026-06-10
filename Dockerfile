FROM rust:1.96-slim-trixie

LABEL org.opencontainers.image.source=https://github.com/PikalaxALT/pokeheartgold-calcrom \
      org.opencontainers.image.description="Rust implementation of calcrom for pret/pokeheartgold" \
      org.opencontainers.image.licenses="MIT"

COPY Cargo.toml /build/Cargo.toml
COPY src /build/src
RUN cargo install --path /build
WORKDIR /
