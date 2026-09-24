/*
 * Copyright 2024 Intel Corporation
 * SPDX-License-Identifier: MIT
 */

#pragma once

#include "dev/intel_device_info.h"

#ifdef __cplusplus
extern "C" {
#endif

static inline uint64_t
intel_canonical_address(uint64_t v)
{
   const int shift = 63 - 47;
   return (int64_t)(v << shift) >> shift;
}

static inline uint64_t
intel_48b_address(uint64_t v)
{
   const int shift = 63 - 47;
   return (uint64_t)(v << shift) >> shift;
}

struct intel_engine_class_instance {
   enum intel_engine_class engine_class;
   uint16_t engine_instance;
   uint16_t gt_id;
};

struct intel_query_engine_info {
   uint32_t num_engines;
   struct intel_engine_class_instance engines[];
};

int intel_engines_count(const struct intel_query_engine_info *info,
                        enum intel_engine_class engine_class);
const char *intel_engines_class_to_string(enum intel_engine_class engine_class);

void
intel_compute_engine_async_threads_limit(const struct intel_device_info *devinfo,
                                         uint32_t hw_threads_in_wg, bool slm_or_barrier_enabled,
                                         bool uses_fence,
                                         uint8_t *ret_pixel_async_compute_thread_limit,
                                         uint8_t *ret_z_pass_async_compute_thread_limit,
                                         uint8_t *ret_np_z_async_throttle_settings);

int
intel_compute_threads_group_dispatch_size(uint32_t hw_threads_in_wg);
int
intel_compute_threads_group_dispatch_size_walker_2(uint32_t hw_threads_in_wg);

/**
 * Convert a number of GRF registers used (grf_used in prog_data) into a
 * number of GRF register blocks supported by the hardware.
 */
unsigned
intel_register_blocks(const struct intel_device_info *devinfo,
                      unsigned grf_used);

/**
 * Returns true if this number of registers can be exactly selected for in
 * hardware.
 */
bool
intel_register_blocks_supported(const struct intel_device_info *devinfo,
                                int num_regs);

uint8_t intel_sampler_state_size(bool uses_efficient_64bit);

#ifdef __cplusplus
}
#endif
