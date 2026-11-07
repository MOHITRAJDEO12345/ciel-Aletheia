FROM rust:1.80-slim AS builder
WORKDIR /usr/src/aletheia
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
LABEL org.opencontainers.image.source="https://github.com/MOHITRAJDEO12345/ananke-Aletheia"
LABEL org.opencontainers.image.description="Ultra-Low-Latency In-Memory Verification Engine for AI Reasoning"
LABEL org.opencontainers.image.licenses="Apache-2.0"

COPY --from=builder /usr/src/aletheia/target/release/aletheia /usr/local/bin/aletheia
RUN chmod +x /usr/local/bin/aletheia

ENTRYPOINT ["/usr/local/bin/aletheia"]
CMD ["benchmark"]

