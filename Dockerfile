# FROM rust:slim as builder
FROM rust:alpine as builder

WORKDIR /build

COPY . /build

RUN cargo build --release && \
    cp target/release/tunelo /usr/bin

# FROM alpine:latest
FROM debian:stable-slim

COPY --from=builder /usr/bin/tunelo /usr/bin/

ENTRYPOINT [ "/usr/bin/tunelo" ]

