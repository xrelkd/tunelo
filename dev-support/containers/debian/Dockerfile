FROM rust:slim as builder

WORKDIR /build

COPY . /build

RUN cargo build --release && \
    cp target/release/tunelo /usr/bin

FROM debian:stable-slim

COPY --from=builder /usr/bin/tunelo /usr/bin/

EXPOSE 3128 3129/udp

ENTRYPOINT [ "/usr/bin/tunelo", "socks-server" ]

