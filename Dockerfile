FROM rust:1.92-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app
RUN apt-get update \
  && apt-get install -y ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/altair-starpaths-ms /app/altair-starpaths-ms

EXPOSE 3005

ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

CMD ["/app/altair-starpaths-ms"]
