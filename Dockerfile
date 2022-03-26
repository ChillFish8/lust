FROM rust:slim-buster as build

WORKDIR /app

COPY . /app

RUN cargo build --release

# Copy the binary into a new container for a smaller docker image
FROM debian:buster-slim

WORKDIR /etc/lust
COPY --from=build /app/target/release/lust /
USER root

ENTRYPOINT ["./lust", "run"]
