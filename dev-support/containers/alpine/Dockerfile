FROM rust:alpine as builder

WORKDIR /build

COPY . /build

# show backtraces
ENV RUST_BACKTRACE 1

RUN apk add --no-cache -U musl-dev binutils file && \
  cargo build --release && \
  strip target/release/tunelo && \
  file target/release/tunelo && \
  cp target/release/tunelo /usr/bin

FROM alpine:latest

RUN apk add --no-cache -U libgcc

COPY --from=builder /usr/bin/tunelo /usr/bin/

EXPOSE 3128 3129/udp

ENTRYPOINT [ "/usr/bin/tunelo", "socks-server" ]

