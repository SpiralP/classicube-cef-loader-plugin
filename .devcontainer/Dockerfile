FROM mcr.microsoft.com/devcontainers/rust:1-bullseye@sha256:6a2d39154cdf1ab57450592916d552cf54b82a01a5365b1a1cc9a2b3fbb117fb

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install \
    # common
    curl ca-certificates git gcc g++ build-essential pkg-config \
    # openssl-sys <- reqwest
    libssl-dev \
    # bindgen <- classicube-sys
    clang-13
