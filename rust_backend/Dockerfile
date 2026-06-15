# ---- Build stage ----
FROM rust:1.90-slim-bookworm AS builder

WORKDIR /build

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates

RUN cargo build --release --locked

# ---- Runtime stage ----
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/* && \
    addgroup --system rust && adduser --system --ingroup rust rust

WORKDIR /app

COPY --from=builder /build/target/release/lista_compras_rust /app/lista_compras_rust
COPY static /app/static
COPY templates /app/templates

RUN chown -R rust:rust /app

USER rust

ENV RUST_PORT=8766
EXPOSE 8766

ENTRYPOINT ["/app/lista_compras_rust"]
