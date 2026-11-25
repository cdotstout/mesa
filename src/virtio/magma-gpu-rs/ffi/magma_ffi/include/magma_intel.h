// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT
//
// Intel vendor-specific Magma query definitions.
// Kept separate from core magma.h so vendor hardware details can evolve
// independently of the core Magma ABI.

#ifndef MAGMA_INTEL_H
#define MAGMA_INTEL_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
    // Returns struct magma_intel_device_info via magma_vendor_query.buffer.
    MAGMA_INTEL_QUERY_DEVICE_INFO = 10000,
    // Returns variable-length uint32_t[] Intel hwconfig table via magma_vendor_query.buffer.
    MAGMA_INTEL_QUERY_HWCONFIG = 10001,
};

typedef uint32_t magma_intel_config_flag_t;
enum {
    MAGMA_INTEL_CONFIG_FLAG_HAS_VRAM = 0x00000001,
    MAGMA_INTEL_CONFIG_FLAG_HAS_LOW_LATENCY = 0x00000002,
    MAGMA_INTEL_CONFIG_FLAG_HAS_CPU_ADDR_MIRROR = 0x00000004,
    MAGMA_INTEL_CONFIG_FLAG_HAS_NO_COMPRESSION_HINT = 0x00000008,
    MAGMA_INTEL_CONFIG_FLAG_HAS_STATE_CACHE_PERF_FIX = 0x00000010,
    MAGMA_INTEL_CONFIG_FLAG_HAS_PURGING_SUPPORT = 0x00000020,
};

struct magma_intel_device_info {
    uint32_t l3_banks;
    uint32_t geo_dss_num_bytes;
    uint64_t geo_dss_mask;
    uint64_t eu_per_dss_mask;
    uint32_t ip_ver_major;
    uint32_t ip_ver_minor;
    uint32_t ip_ver_rev;
    magma_intel_config_flag_t config_flags;
};

#ifdef __cplusplus
}
#endif

#endif
