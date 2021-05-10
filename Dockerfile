FROM rust:1.50 as build

WORKDIR /sse

COPY . .

RUN cargo build --release --bin se-stats-exporter

ENTRYPOINT ["/sse/target/release/se-stats-exporter"]

#FROM scratch
#
#COPY --from=build /sse/target/release/se-stats-exporter /bin/se-stats-exporter
#
#ENTRYPOINT ["/bin/se-stats-exporter"]
