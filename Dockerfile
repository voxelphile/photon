FROM rust

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release --bin photon

CMD ["./target/release/photon"]