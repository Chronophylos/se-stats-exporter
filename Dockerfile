FROM rust:1.50

WORKDIR /usr/src/tbc
COPY . .

RUN cargo install --path .
