FROM docker.io/library/debian:trixie

ENV DEBIAN_FRONTEND=noninteractive

RUN dpkg --add-architecture i386 && \
    apt-get update && apt-get install -y --no-install-recommends \
    bison \
    bindgen \
    build-essential \
    ca-certificates \
    clang-19 \
    cmake \
    curl \
    flex \
    g++-multilib \
    gcc-multilib \
    git \
    glslang-tools \
    libclang-19-dev \
    libclang-common-19-dev \
    libclang-cpp19-dev \
    libclc-19 \
    libclc-19-dev \
    libdrm-dev \
    libdrm-dev:i386 \
    libelf-dev \
    libelf-dev:i386 \
    libexpat1-dev \
    libexpat1-dev:i386 \
    libllvmspirvlib-19-dev \
    libpciaccess-dev \
    libpciaccess-dev:i386 \
    libvulkan-dev \
    libvulkan-dev:i386 \
    libwayland-bin \
    libwayland-dev \
    libwayland-dev:i386 \
    libwayland-egl-backend-dev \
    libwayland-egl-backend-dev:i386 \
    libx11-dev \
    libx11-dev:i386 \
    libx11-xcb-dev \
    libx11-xcb-dev:i386 \
    libxcb-dri2-0-dev \
    libxcb-dri2-0-dev:i386 \
    libxcb-dri3-dev \
    libxcb-dri3-dev:i386 \
    libxcb-glx0-dev \
    libxcb-glx0-dev:i386 \
    libxcb-keysyms1-dev \
    libxcb-keysyms1-dev:i386 \
    libxcb-present-dev \
    libxcb-present-dev:i386 \
    libxcb-randr0-dev \
    libxcb-randr0-dev:i386 \
    libxcb-shm0-dev \
    libxcb-shm0-dev:i386 \
    libxcb-sync-dev \
    libxcb-sync-dev:i386 \
    libxcb-xfixes0-dev \
    libxcb-xfixes0-dev:i386 \
    libxcb1-dev \
    libxcb1-dev:i386 \
    libxext-dev \
    libxext-dev:i386 \
    libxrandr-dev \
    libxrandr-dev:i386 \
    libxrender-dev \
    libxrender-dev:i386 \
    libxshmfence-dev \
    libxshmfence-dev:i386 \
    libxxf86vm-dev \
    libxxf86vm-dev:i386 \
    libzstd-dev \
    libzstd-dev:i386 \
    llvm-19-dev \
    llvm-spirv-19 \
    meson \
    mingw-w64 \
    mingw-w64-tools \
    ninja-build \
    pkg-config \
    pkgconf \
    python3 \
    python3-mako \
    python3-packaging \
    python3-pip \
    python3-ply \
    python3-pycparser \
    python3-yaml \
    spirv-tools \
    wayland-protocols \
    wine \
    wine64 \
    x11proto-dri2-dev \
    x11proto-gl-dev \
    x11proto-randr-dev \
    xz-utils \
    zlib1g-dev \
    zlib1g-dev:i386 \
    zstd \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH="/mesa/build-linux/src/compiler/clc:/mesa/build-linux/src/compiler/spirv:/usr/local/cargo/bin:${PATH}"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
        --default-toolchain 1.88.0 \
        --target x86_64-unknown-linux-gnu \
        --target i686-unknown-linux-gnu \
        --target x86_64-pc-windows-gnu && \
    chmod -R a+rX /usr/local/rustup /usr/local/cargo

RUN cat <<'EOF' > /cross_file_i386_linux_gnu.txt
[binaries]
c = ['gcc', '-m32']
cpp = ['g++', '-m32']
rust = ['rustc', '--target', 'i686-unknown-linux-gnu', '-C', 'linker=gcc', '-C', 'link-arg=-m32']
ar = 'gcc-ar'
strip = 'strip'
pkg-config = 'pkg-config'

[properties]
pkg_config_libdir = '/usr/lib/i386-linux-gnu/pkgconfig:/usr/share/pkgconfig'

[host_machine]
system = 'linux'
cpu_family = 'x86'
cpu = 'i686'
endian = 'little'
EOF

RUN cat <<'EOF' > /cross_file_x86_64_linux_mingw.txt
[binaries]
c = 'x86_64-w64-mingw32-gcc'
cpp = 'x86_64-w64-mingw32-g++'
ar = 'x86_64-w64-mingw32-ar'
nm = 'x86_64-w64-mingw32-nm'
strip = 'x86_64-w64-mingw32-strip'
windres = 'x86_64-w64-mingw32-windres'
pkg-config = 'x86_64-w64-mingw32-pkg-config'
rust = ['rustc', '--target', 'x86_64-pc-windows-gnu', '-C', 'linker=x86_64-w64-mingw32-gcc']

[host_machine]
system = 'windows'
cpu_family = 'x86_64'
cpu = 'x86_64'
endian = 'little'
EOF

WORKDIR /mesa
