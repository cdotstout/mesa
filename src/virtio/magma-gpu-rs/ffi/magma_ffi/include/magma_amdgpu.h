// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT
//
// AMDGPU vendor-specific Magma query definitions.

#ifndef MAGMA_AMDGPU_H
#define MAGMA_AMDGPU_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
    // Returns struct magma_amdgpu_device_info via magma_vendor_query.buffer.
    MAGMA_AMDGPU_QUERY_DEVICE_INFO = 10000,
};

struct magma_amdgpu_device_info {
    uint64_t max_engine_clock;
    uint64_t max_memory_clock;
    uint64_t ids_flags;
    uint32_t chip_rev;
    uint32_t external_rev;
    uint32_t family;
    uint32_t num_shader_engines;
    uint32_t num_shader_arrays_per_engine;
    uint32_t cu_active_number;
    uint32_t enabled_rb_pipes_mask;
    uint32_t num_rb_pipes;
    uint32_t vram_type;
    uint32_t vram_bit_width;
    uint32_t wave_front_size;
    uint32_t num_shader_visible_vgprs;
    uint32_t num_cu_per_sh;
    uint32_t num_tcc_blocks;
};

#ifdef __cplusplus
}
#endif

#endif
