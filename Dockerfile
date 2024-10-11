FROM rust:1.81 as builder

RUN apt-get update && apt-get install -y \
    musl-tools \
    pkg-config \
    curl \
    build-essential \
    musl-dev \
    libssl-dev \
    ca-certificates \
    && update-ca-certificates

WORKDIR /opt
COPY . .

RUN curl -LO https://www.openssl.org/source/openssl-1.1.1w.tar.gz && \
    tar -xzf openssl-1.1.1w.tar.gz && \
    cd openssl-1.1.1w && \
    ./Configure no-shared --prefix=/usr/local/musl/openssl linux-x86_64 && \
    make && make install

RUN rustup target add x86_64-unknown-linux-musl
ENV OPENSSL_DIR=/usr/local/musl/openssl
ENV OPENSSL_STATIC=1
ENV PKG_CONFIG_ALLOW_CROSS=1

RUN cargo build --target x86_64-unknown-linux-musl --release

FROM alpine:latest
RUN apk add --no-cache ca-certificates

WORKDIR /root/

COPY --from=builder /opt/target/x86_64-unknown-linux-musl/release/selfmqr .
COPY --from=builder /opt/.env .

CMD ["./selfmqr"]