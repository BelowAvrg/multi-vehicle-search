
FROM rust:1.85-slim-bullseye AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release --bin multi-vehicle-search

FROM debian:bullseye-slim
WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/multi-vehicle-search /usr/local/bin/

ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

ENTRYPOINT ["multi-vehicle-search"]