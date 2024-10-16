# docker build --progress plain --pull -t cross-aarch64 -f .github/cross-aarch64.Dockerfile . && cross build --target aarch64-unknown-linux-gnu

# ubuntu-like
# we need at least glibc 2.29
# 0.2.4 and 0.2.5 have glibc 2.23, so use main which has glibc 2.31
# https://github.com/cross-rs/cross/pkgs/container/aarch64-unknown-linux-gnu
FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main

# we're root
RUN export DEBIAN_FRONTEND=noninteractive \
    && dpkg --add-architecture arm64 \
    && apt-get -y update \
    && apt-get -y install wget curl git gcc g++ build-essential cmake clang pkg-config \
    gcc-aarch64-linux-gnu g++-aarch64-linux-gnu libc6-dev-i386 \
    libssl-dev:arm64 \
    libssl1.1:arm64 \
    && apt-get -y autoremove && apt-get -y clean && rm -rf /var/lib/apt \
    && rm -rf /tmp && mkdir /tmp && chmod 777 /tmp \
    && rm -rf /usr/share/doc /usr/share/man /usr/share/locale

ENV PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig
ENV CPLUS_INCLUDE_PATH=/usr/aarch64-linux-gnu/include/c++/5/aarch64-linux-gnu

# bits/c++config.h not found
RUN ln -vs /usr/aarch64-linux-gnu/include/c++/9/aarch64-linux-gnu/bits/* /usr/include/c++/9/bits/
