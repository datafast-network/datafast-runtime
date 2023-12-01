FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY .. .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY .. .
RUN cargo build --release

FROM debian:bookworm-slim as runtime
RUN apt-get update && apt install apt-transport-https ca-certificates gnupg openssl -y && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/subgraph-wasm-runtime /usr/local/bin/subgraph_runtime

WORKDIR app

EXPOSE 8081

CMD ["/usr/local/bin/subgraph_runtime"]
