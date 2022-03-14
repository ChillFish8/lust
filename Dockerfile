FROM rust:slim-buster as build

WORKDIR /code

COPY . /code

RUN cargo build --release

# Copy the binary into a new container for a smaller docker image
FROM debian:buster-slim

WORKDIR /etc/lust
COPY --from=build /code/target/release/lust /
USER root

ENTRYPOINT ["./lust", "run"]
