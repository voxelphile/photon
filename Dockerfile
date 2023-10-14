FROM rust

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release --bin photon

EXPOSE 5432
ENV DB_HOST=34.118.225.0:5432
ENV PHOTON_STRATEGY=host

CMD ["./target/release/photon"]