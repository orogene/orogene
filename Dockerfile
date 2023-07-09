FROM rust:1-bookworm
WORKDIR /usr/src/orogene
COPY . .
RUN cargo install --path .
CMD ["orogene"]
