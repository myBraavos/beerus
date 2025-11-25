FROM rust:1.90-slim-bullseye AS builder
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y \
    libssl-dev \
    pkg-config \
    libpq-dev
WORKDIR /beerus

# Copy dependency files first
COPY Cargo.toml Cargo.lock ./

# Create dummy source files to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached unless Cargo.toml/Cargo.lock changes)
RUN CARGO_BUILD_JOBS=$(nproc) \
    cargo build --release --bin beerus && rm -rf src

# Copy the actual source code
COPY src/ ./src/

# Build the actual binary (only rebuilds if source code changes)
RUN CARGO_BUILD_JOBS=$(nproc) \
    cargo build --release --bin beerus
RUN strip target/release/beerus

FROM debian:bullseye-slim
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y ca-certificates
COPY --from=builder /beerus/target/release/beerus /usr/local/bin/

EXPOSE 3030

LABEL description="Starknet Light Client"
LABEL source="https://github.com/myBraavos/beerus"

ENTRYPOINT ["/usr/local/bin/beerus"]
