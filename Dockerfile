FROM rust:1-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Cache dependencies before copying source
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src benches && \
    echo 'fn main() {}' > src/main.rs && \
    echo '' > benches/query_layer_bench.rs && \
    cargo build --release && \
    rm -rf src benches

COPY . .
# Touch main.rs so cargo detects the source changed
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/merk /app/merk

EXPOSE 9678

CMD ["/app/merk"]
