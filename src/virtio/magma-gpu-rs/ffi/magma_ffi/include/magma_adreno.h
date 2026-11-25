// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT
//
// Qualcomm Adreno vendor-specific Magma query definitions.

#ifndef MAGMA_ADRENO_H
#define MAGMA_ADRENO_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
    // Returns struct magma_adreno_device_info via magma_vendor_query.buffer.
    MAGMA_ADRENO_QUERY_DEVICE_INFO = 10000,
};

struct magma_adreno_device_info {
    uint64_t chip_id;
    uint64_t gmem_size;
    uint64_t gmem_base;
    uint32_t gpu_id;
    uint32_t highest_bank_bit;
    uint32_t ubwc_swizzle;
    uint32_t macrotile_mode;
    uint32_t raytracing;
    uint32_t flags;
};

#ifdef __cplusplus
}
#endif

#endif
