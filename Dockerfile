FROM rust:1.92-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

COPY --from=builder /app/target/release/altair-starpaths-ms /app/altair-starpaths-ms

EXPOSE 3005

ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

USER nonroot:nonroot

CMD ["/app/altair-starpaths-ms"]
