FROM debian:bookworm

RUN export DEBIAN_FRONTEND=noninteractive \
  && apt-get -y update \
  && apt-get -y install --no-install-recommends \
  # common
  curl ca-certificates git gcc g++ build-essential pkg-config \
  # openssl-sys <- reqwest
  libssl-dev \
  # bindgen <- classicube-sys
  clang-10 \
  && curl -sS 'https://sh.rustup.rs' | sh -s -- -y --default-toolchain none \
  && ~/.cargo/bin/rustup default nightly \
  && ~/.cargo/bin/rustup component add rust-src rustfmt clippy
