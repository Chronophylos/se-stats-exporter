FROM rust:1.50 as build

WORKDIR /sse
COPY . .

RUN cargo build --release --bin se-stats-exporter

FROM scratch

COPY --from=build /sse/target/release/se-stats-exporter /usr/local/bin

ENTRYPOINT ["/usr/local/bin/se-stats-exporter"]