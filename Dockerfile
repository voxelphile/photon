FROM rust

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release --bin photon

EXPOSE 5432

CMD ["./target/release/photon"]