FROM rust:latest as builder

RUN mkdir /builder
WORKDIR /builder

COPY . .

RUN cargo build --release
RUN cp ./target/release/lust .

FROM alpine:latest as publish
COPY --from=builder /builder/lust /lust

ENTRYPOINT ["/lust", "run"]