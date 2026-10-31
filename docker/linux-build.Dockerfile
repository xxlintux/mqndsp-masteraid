# Reproducible Linux release build.
#
# Uses Ubuntu 22.04 (glibc 2.35) so the resulting plugin runs on 22.04 and
# newer distributions (Debian 12, Arch, Fedora 39/40).
# Builds packages for:
#   - Universal (.tar.gz)
#   - Debian / Ubuntu (.deb)
#   - Arch Linux (.pkg.tar.zst)
#   - Fedora / RHEL (.rpm)
#
# Build the image from the repository root:
#   docker build -f docker/linux-build.Dockerfile -t mqn-masteraid-linux-build .
#   docker run --rm -v "$PWD:/src" -w /src mqn-masteraid-linux-build sh ./scripts/package-linux.sh
#
# Or use scripts/build-linux-docker.sh.
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        git \
        pkg-config \
        python3 \
        rpm \
        zstd \
        dpkg-dev \
        libx11-dev \
        libx11-xcb-dev \
        libxcb1-dev \
        libxcb-icccm4-dev \
        libxcb-util-dev \
        libxcursor-dev \
        libgl1-mesa-dev \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /src
CMD ["sh", "./scripts/package-linux.sh"]
