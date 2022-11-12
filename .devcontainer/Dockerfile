FROM mcr.microsoft.com/devcontainers/rust:1-bullseye

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install \
    # common
    curl ca-certificates git gcc g++ build-essential pkg-config \
    # openssl-sys <- reqwest
    libssl-dev \
    # bindgen <- classicube-sys
    clang-13
