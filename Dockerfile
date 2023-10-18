FROM rust

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release --bin photon

EXPOSE 5432
EXPOSE 6379
ENV DB_HOST=10.76.1.0:5432
ENV REDIS_HOST=10.76.3.0:6379

CMD ["./target/release/photon"]