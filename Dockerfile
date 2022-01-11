FROM rust:1.57-slim

RUN apt update
RUN apt install -y libzmq3-dev pkg-config

WORKDIR /app

# to build image:
# docker build -t webx-router-builder .
# docker run --rm --user "$(id -u)":"$(id -g)" -v "$PWD":/app webx-router-builder cargo build --release
# Built executable can be found in target/release