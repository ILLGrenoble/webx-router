FROM rust:1.58-slim

WORKDIR /app

RUN apt update
RUN apt install -y libzmq3-dev pkg-config dpkg-dev

COPY . .

RUN cargo install cargo-deb
RUN cargo deb

# Save the version to a file
RUN awk -F ' = ' '$1 ~ /version/ { gsub(/[\"]/, "", $2); printf("%s",$2) }' Cargo.toml > VERSION

# to obtain built deb package:
# docker build -t webx-router-builder .
# docker create -ti --name webx-router-builder webx-router-builder bash
# docker cp webx-router-builder:/app/target/debian/. .
# docker rm -f webx-router-builder
