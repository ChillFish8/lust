FROM rust:latest as builder

RUN mkdir /builder
WORKDIR /builder

COPY . .

RUN cargo build --release
RUN cp ./target/release/lust .

FROM alpine:latest as publish
WORKDIR /app

COPY --from=builder /builder/lust /app
RUN ls /app

ENTRYPOINT ["/app/lust", "run"]