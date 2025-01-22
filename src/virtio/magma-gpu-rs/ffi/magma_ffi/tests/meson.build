# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

magma_ffi_test = executable(
  'magma_ffi_test',
  'magma_ffi_test.c',
  include_directories : [include_directories('../include')],
  link_with : [libmesa_magma_ffi],
  dependencies : [dep_thread, dep_dl, dep_m],
)

test(
  'magma_ffi_test',
  magma_ffi_test,
  suite : ['magma'],
)
