FROM rust:1-bookworm as builder
WORKDIR /usr/src/orogene
COPY . .
RUN cargo install --path .
FROM scratch
COPY --from=builder /usr/src/orogene/target/release/oro /oro
CMD ["oro"]
