FROM rust:1.81 as builder

WORKDIR /opt
COPY . .

RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --target x86_64-unknown-linux-musl --release
FROM alpine:latest  
RUN apk add --no-cache ca-certificates

WORKDIR /root/

COPY --from=builder /opt/target/x86_64-unknown-linux-musl/release/selfmqr .
COPY --from=builder /opt/.env .

CMD ["./selfmqr"]