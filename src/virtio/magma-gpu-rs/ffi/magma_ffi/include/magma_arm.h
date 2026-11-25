// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT
//
// ARM Mali vendor-specific Magma query definitions.

#ifndef MAGMA_ARM_H
#define MAGMA_ARM_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
    // Returns struct magma_arm_device_info via magma_vendor_query.buffer.
    MAGMA_ARM_QUERY_DEVICE_INFO = 10000,
};

struct magma_arm_device_info {
    uint64_t shader_present;
    uint64_t tiler_present;
    uint64_t l2_present;
    uint32_t gpu_id;
    uint32_t gpu_rev;
    uint32_t csf_id;
    uint32_t l2_features;
    uint32_t tiler_features;
    uint32_t mem_features;
    uint32_t mmu_features;
    uint32_t thread_features;
    uint32_t max_threads;
    uint32_t thread_max_workgroup_size;
    uint32_t thread_max_barrier_size;
    uint32_t coherency_features;
    uint32_t selected_coherency;
    uint32_t as_present;
    uint32_t csg_slot_count;
    uint32_t cs_slot_count;
    uint32_t cs_reg_count;
    uint32_t scoreboard_slot_count;
};

#ifdef __cplusplus
}
#endif

#endif
