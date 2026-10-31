#!/usr/bin/env sh
# Builds the Linux package inside an Ubuntu 22.04 container, so the artifact
# works on glibc 2.35+ regardless of the host distribution.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
IMAGE="mqn-masteraid-linux-build"

docker build -f "$ROOT/docker/linux-build.Dockerfile" -t "$IMAGE" "$ROOT"
docker run --rm -v "$ROOT:/src" -w /src "$IMAGE" sh ./scripts/package-linux.sh
