# Multi-stage build for the generated hexagonal REST API.
# v1.0.0 (closes G-M19): the CMD no longer uses `ls | head -1` shell
# glob — the binary name is templated from the project name at
# codegen time so the CMD is a literal exec, which is both safer
# (no shell injection surface) and faster (no fork+exec of sh).
FROM rust:1.82-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/generated_api /usr/local/bin/generated_api
EXPOSE 8080
ENV RUST_LOG=info
CMD ["/usr/local/bin/generated_api"]
