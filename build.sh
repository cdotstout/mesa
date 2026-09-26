# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

set -e

IMAGE_NAME="mesa-builder:latest"
TTY_FLAGS=""
if [ -t 0 ]; then
    TTY_FLAGS="-it"
fi

run_in_container() {
    podman run --rm ${TTY_FLAGS} \
        --userns=keep-id \
        -v "$(pwd)":/mesa:z \
        "${IMAGE_NAME}" \
        "$@"
}

build_image() {
    if ! podman image exists "${IMAGE_NAME}"; then
        podman build -t "${IMAGE_NAME}" - < Containerfile
    fi
}

build_linux() {
    build_image
    if [ ! -f build-linux/build.ninja ]; then
        run_in_container meson setup build-linux /mesa \
            -Dvulkan-drivers=intel \
            -Dgallium-drivers= \
            -Dmagma=true \
            -Dopengl=false \
            -Drust_std=2021 \
            -Dbuild-tests=true \
            -Dmesa-clc=enabled
    fi
    run_in_container ninja -C build-linux
}

build_i386() {
    build_image
    if [ ! -f build-i386/build.ninja ]; then
        run_in_container meson setup build-i386 /mesa \
            --cross-file /cross_file_i386_linux_gnu.txt \
            -Dvulkan-drivers=intel \
            -Dgallium-drivers= \
            -Dmagma=true \
            -Dopengl=false \
            -Drust_std=2021 \
            -Dbuild-tests=false \
            -Dmesa-clc=system
    fi
    run_in_container ninja -C build-i386
}

build_windows() {
    build_image
    if [ ! -f build-windows/build.ninja ]; then
        run_in_container meson setup build-windows /mesa \
            --cross-file /cross_file_x86_64_linux_mingw.txt \
            -Dplatforms=windows \
            -Dvulkan-drivers=intel \
            -Dgallium-drivers= \
            -Dmagma=true \
            -Dopengl=false \
            -Degl=disabled \
            -Drust_std=2021 \
            -Dbuild-tests=false \
            -Dmesa-clc=system
    fi
    run_in_container ninja -C build-windows
}

case "${1:-all}" in
    image)
        podman build -t "${IMAGE_NAME}" - < Containerfile
        ;;
    linux)
        build_linux
        ;;
    i386)
        build_i386
        ;;
    windows)
        build_windows
        ;;
    all)
        build_linux
        build_i386
        build_windows
        ;;
    *)
        echo "Usage: $0 [all|image|linux|i386|windows]" >&2
        exit 1
        ;;
esac
