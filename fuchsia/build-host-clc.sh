#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT_DIR}/host-clc-build"
STAGING_DIR="${1:-${ROOT_DIR}/host-clc-staging}"

meson setup --reconfigure "${BUILD_DIR}" "${ROOT_DIR}" \
  -Dprefix="${STAGING_DIR}" \
  -Dbuildtype=release \
  -Dstrip=true \
  -Dplatforms= \
  -Dgallium-drivers= \
  -Dvulkan-drivers= \
  -Dopengl=false \
  -Dmesa-clc=enabled \
  -Dinstall-mesa-clc=true

meson install -C "${BUILD_DIR}"
