## -- Debian 11
FROM rust:1-bullseye-slim AS debian11

WORKDIR /app

RUN apt update
RUN apt install -y libzmq3-dev pkg-config dpkg-dev

COPY . .

RUN cargo install cargo-deb
RUN cargo deb


## -- Debian 12
FROM rust:1-bookworm-slim AS debian12

WORKDIR /app

RUN apt update
RUN apt install -y libzmq3-dev pkg-config dpkg-dev

COPY . .

RUN cargo install cargo-deb
RUN cargo deb


## -- Ubuntu 22.04
FROM ubuntu:22.04 AS ubuntu22

WORKDIR /app

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


## -- Ubuntu 24.04
FROM ubuntu:24.04 AS ubuntu24

WORKDIR /app

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
RUN mkdir -p target/debian11
RUN mkdir -p target/debian12
RUN mkdir -p target/ubuntu22
RUN mkdir -p target/ubuntu24
COPY --from=debian11 /app/packages/* target/debian11
COPY --from=debian12 /app/packages/* target/debian12
COPY --from=ubuntu22 /app/packages/* target/ubuntu22
COPY --from=ubuntu24 /app/packages/* target/ubuntu24

# to obtain built deb package:
# docker build -t webx-router-builder .
# docker create -ti --name webx-router-builder webx-router-builder bash
# docker cp webx-router-builder:/app/target/debian/. .
# docker rm -f webx-router-builder
