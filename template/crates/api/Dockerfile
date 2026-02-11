FROM rust:1.93-slim as builder
ARG PREFIX=.
RUN apt-get update && \
    apt-get install -y curl pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cd ${PREFIX}/ && cargo build --release --bin api
RUN mv ${PREFIX}/target/release/api api

FROM gcr.io/distroless/cc
ARG PREFIX=.
WORKDIR /app
# COPY ${PREFIX}/microkit.yml .
COPY --from=builder /app/api /app/api
CMD ["/app/api"]
