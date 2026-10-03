#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
BUILD_DIR="${1:-${ROOT_DIR}/build-fuchsia}"
STAGING_DIR="${ROOT_DIR}/host-clc-staging"

if [ ! -x "${STAGING_DIR}/bin/mesa_clc" ] || [ ! -x "${STAGING_DIR}/bin/vtn_bindgen2" ]; then
  echo "Building host clc tools..."
  "${SCRIPT_DIR}/build-host-clc.sh" "${STAGING_DIR}"
fi

if [ ! -d "${ROOT_DIR}/subprojects/fuchsia-sdk" ]; then
  if [ ! -f "${SCRIPT_DIR}/downloads/core-linux-amd64.zip" ]; then
    echo "Missing ${SCRIPT_DIR}/downloads/core-linux-amd64.zip, please download from:"
    echo "https://chrome-infra-packages.appspot.com/p/fuchsia/sdk/core/linux-amd64"
    exit 1
  fi
  echo "Unzipping Fuchsia SDK..."
  mkdir -p "${ROOT_DIR}/subprojects/fuchsia-sdk"
  unzip -q "${SCRIPT_DIR}/downloads/core-linux-amd64.zip" -d "${ROOT_DIR}/subprojects/fuchsia-sdk"
fi

if [ ! -d "${SCRIPT_DIR}/tools/clang" ]; then
  if [ ! -f "${SCRIPT_DIR}/downloads/clang-linux-amd64.zip" ]; then
    echo "Missing ${SCRIPT_DIR}/downloads/clang-linux-amd64.zip, please download from:"
    echo "https://chrome-infra-packages.appspot.com/p/fuchsia/third_party/clang/linux-amd64"
    exit 1
  fi
  echo "Unzipping Fuchsia Clang..."
  mkdir -p "${SCRIPT_DIR}/tools/clang"
  unzip -q "${SCRIPT_DIR}/downloads/clang-linux-amd64.zip" -d "${SCRIPT_DIR}/tools/clang"
fi

echo "Generating Meson build definition for Fuchsia SDK..."
"${SCRIPT_DIR}/create_meson.py" "${ROOT_DIR}/subprojects/fuchsia-sdk"

export PATH="${STAGING_DIR}/bin:${HOME}/.cargo/bin:${PATH}"

echo "Configuring Meson build..."
meson setup --reconfigure "${BUILD_DIR}" "${ROOT_DIR}" \
  --cross-file "${SCRIPT_DIR}/fuchsia-cross.ini" \
  -Dbuildtype=debug \
  -Dvulkan-drivers="intel" \
  -Dgallium-drivers="" \
  -Dplatforms="" \
  -Dmagma=true \
  -Dopengl=false \
  -Degl=disabled \
  -Dglx=disabled \
  -Dshader-cache=disabled \
  -Dzlib=disabled \
  -Dzstd=disabled \
  -Dxmlconfig=disabled \
  -Dexpat=disabled \
  -Dllvm=disabled \
  -Dmesa-clc=system \
  -Drust_std=2021 \
  -Dbuild-tests=true

echo "Running Ninja build..."
ninja -C "${BUILD_DIR}"
