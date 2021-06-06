FROM ekidd/rust-musl-builder as builder

WORKDIR /home/rust/

# Avoid having to install/build all dependencies by copying
# the Cargo files and making a dummy src/main.rs
COPY . .
RUN cargo build --release

# Size optimization
RUN strip target/x86_64-unknown-linux-musl/release/lust

# Start building the final image
FROM scratch
WORKDIR /etc/lust

COPY --from=builder /home/rust/target/x86_64-unknown-linux-musl/release/lust .
ENTRYPOINT ["./lust", "run"]