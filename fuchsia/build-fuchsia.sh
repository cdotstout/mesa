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
MESON_ARGS=()
if [ -f "${BUILD_DIR}/build.ninja" ]; then
  MESON_ARGS+=(--reconfigure --clearcache)
fi

meson setup "${MESON_ARGS[@]}" "${BUILD_DIR}" "${ROOT_DIR}" \
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
  -Dlibunwind=disabled \
  -Dmesa-clc=system \
  -Drust_std=2021 \
  -Dbuild-tests=true

echo "Running Ninja build..."
ninja -C "${BUILD_DIR}"

SDK_DIR="${ROOT_DIR}/subprojects/fuchsia-sdk"
CLANG_BIN="${ROOT_DIR}/fuchsia/tools/clang/bin"
PKG_NAME="vulkan_intel"
ICD_SO_NAME="libvulkan_intel.so"
UNSTRIPPED_SO="${BUILD_DIR}/src/intel/vulkan/${ICD_SO_NAME}"
PKG_WORK_DIR="${BUILD_DIR}/${PKG_NAME}_pkg"

rm -rf "${PKG_WORK_DIR}"
mkdir -p "${PKG_WORK_DIR}/lib" "${PKG_WORK_DIR}/meta/icd.d" "${PKG_WORK_DIR}/meta/metadata" "${PKG_WORK_DIR}/out"

"${CLANG_BIN}/llvm-strip" --strip-all "${UNSTRIPPED_SO}" -o "${PKG_WORK_DIR}/lib/${ICD_SO_NAME}"

BUILD_ID="$("${CLANG_BIN}/llvm-readelf" -n "${UNSTRIPPED_SO}" | awk '/Build ID:/ { print $3; exit }')"
if [ -n "${BUILD_ID}" ]; then
  SYM_DIR="${PKG_WORK_DIR}/symbols/.build-id/${BUILD_ID:0:2}"
  mkdir -p "${SYM_DIR}"
  cp -f "${UNSTRIPPED_SO}" "${SYM_DIR}/${BUILD_ID:2}.debug"
  tar -C "${PKG_WORK_DIR}/symbols" -cf "${BUILD_DIR}/${PKG_NAME}_symbols.tar" .
fi

printf '{"name":"%s","version":"0"}\n' "${PKG_NAME}" > "${PKG_WORK_DIR}/meta/package"

cat > "${PKG_WORK_DIR}/meta/icd.d/${ICD_SO_NAME}.json" <<EOF
{
  "file_format_version": "1.0.0",
  "ICD": {
    "library_path": "${ICD_SO_NAME}",
    "api_version": "1.1.0"
  }
}
EOF

cat > "${PKG_WORK_DIR}/meta/metadata/metadata.json" <<EOF
{
  "file_path": "lib/${ICD_SO_NAME}",
  "library_path": "${ICD_SO_NAME}",
  "version": 1,
  "manifest_path": "meta/icd.d/${ICD_SO_NAME}.json"
}
EOF

"${SDK_DIR}/tools/x64/cmc" compile "${SCRIPT_DIR}/meta/vulkan.cml" \
  --includepath "${SDK_DIR}/pkg" \
  --output "${PKG_WORK_DIR}/meta/${PKG_NAME}.cm"

cat > "${PKG_WORK_DIR}/${PKG_NAME}.manifest" <<EOF
lib/${ICD_SO_NAME}=${PKG_WORK_DIR}/lib/${ICD_SO_NAME}
meta/package=${PKG_WORK_DIR}/meta/package
meta/${PKG_NAME}.cm=${PKG_WORK_DIR}/meta/${PKG_NAME}.cm
meta/icd.d/${ICD_SO_NAME}.json=${PKG_WORK_DIR}/meta/icd.d/${ICD_SO_NAME}.json
meta/metadata/metadata.json=${PKG_WORK_DIR}/meta/metadata/metadata.json
EOF

"${SDK_DIR}/tools/x64/ffx_tools/ffx-package" package build \
  "${PKG_WORK_DIR}/${PKG_NAME}.manifest" \
  --api-level 32 \
  --out "${PKG_WORK_DIR}/out"

"${SDK_DIR}/tools/x64/ffx_tools/ffx-package" package archive create \
  "${PKG_WORK_DIR}/out/package_manifest.json" \
  --out "${BUILD_DIR}/${PKG_NAME}.far"

