FROM rust:1-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY benches ./benches
RUN cargo build --release --bins

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/pdf-maldives-be ./
COPY --from=builder /app/target/release/pdf_worker ./
COPY libpdfium.so ./
ENV LD_LIBRARY_PATH=/app
EXPOSE 4000
CMD ["./pdf-maldives-be"]
