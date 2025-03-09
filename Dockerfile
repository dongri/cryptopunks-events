FROM rust:1.85 as builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cryptopunks-events /usr/local/bin/cryptopunks-events

CMD ["cryptopunks-events"]
