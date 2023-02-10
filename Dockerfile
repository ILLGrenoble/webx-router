FROM rust:1.58-slim AS debian

WORKDIR /app

RUN apt update
RUN apt install -y libzmq3-dev pkg-config dpkg-dev

COPY . .

RUN cargo install cargo-deb
RUN cargo deb

# Save the version to a file
RUN awk -F ' = ' '$1 ~ /version/ { gsub(/[\"]/, "", $2); printf("%s",$2) }' Cargo.toml > VERSION

FROM ubuntu:22.04 AS ubuntu

WORKDIR /app

# Install package dependencies.
RUN apt-get update
RUN apt install -y apt-utils curl gcc libzmq3-dev pkg-config dpkg-dev libclang-dev libpam-dev clang

# Install Rust
RUN curl https://sh.rustup.rs -sSf > /tmp/rustup-init.sh \
    && chmod +x /tmp/rustup-init.sh \
    && sh /tmp/rustup-init.sh -y \
    && rm -rf /tmp/rustup-init.sh

COPY . .

RUN ~/.cargo/bin/cargo install cargo-deb
RUN ~/.cargo/bin/cargo deb

FROM alpine:3

WORKDIR /app

# Copy package to standard directory
RUN mkdir -p target/debian
RUN mkdir -p target/ubuntu
COPY --from=debian /app/target/debian/* target/debian/
COPY --from=ubuntu /app/target/debian/* target/ubuntu/
COPY --from=debian /app/VERSION .

# to obtain built deb package:
# docker build -t webx-router-builder .
# docker create -ti --name webx-router-builder webx-router-builder bash
# docker cp webx-router-builder:/app/target/debian/. .
# docker rm -f webx-router-builder
