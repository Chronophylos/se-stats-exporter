FROM rust:1.52 as builder

WORKDIR /sse

COPY . .

RUN cargo install -v --bin se-stats-exporter --path .

FROM debian:buster-slim

RUN apt-get update && apt-get install -y openssl && rm -rf /var/lib/apt/lists/*

EXPOSE 9001/tcp

COPY --from=builder /usr/local/cargo/bin/se-stats-exporter /bin/se-stats-exporter

ENTRYPOINT ["/bin/se-stats-exporter"]
