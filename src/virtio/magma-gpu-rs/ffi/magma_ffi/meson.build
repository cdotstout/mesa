# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

dep_libc_rs = dependency('rust-libc',
  version: '>= 0.2.185',
  fallback: ['libc-0.2-rs', 'dep_libc'],
  required: true,
)

libmesa_magma_ffi = static_library(
  'mesa3d_magma_ffi',
  'lib.rs',
  gnu_symbol_visibility : 'hidden',
  rust_abi : 'c',
  link_with: [libmesa_magma, libmagma_gpu_rs],
  dependencies: [dep_magma_gpu_rs, dep_log, dep_libc_rs]
)

if with_tests
  subdir('tests')
endif

inc_magma_ffi = include_directories('include')

idep_magma_ffi = declare_dependency(
  link_with: libmesa_magma_ffi,
  include_directories: inc_magma_ffi,
)
