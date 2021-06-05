FROM rust:latest as builder

RUN mkdir /builder
WORKDIR /builder

COPY . .

RUN cargo build --release
RUN cp ./target/release/lust ./lust
RUN rm -rf ./target

ENTRYPOINT ["./lust", "run"]