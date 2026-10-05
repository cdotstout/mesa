/*
 * Copyright © 2023 Intel Corporation
 * SPDX-License-Identifier: MIT
 */

#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "vulkan/vulkan_core.h"

#include "dev/intel_device_info.h"
#include "dev/intel_kmd.h"
#include "isl/isl.h"

struct _drmDevice;
struct anv_bo;
enum anv_bo_alloc_flags;
struct anv_device;
struct anv_physical_device;
struct anv_queue;
struct vk_instance;
struct vk_physical_device;

struct anv_linux_sys_driver_ops {
   VkResult (*get_bo_tiling)(struct anv_device *device, struct anv_bo *bo,
                             enum isl_tiling *tiling_out);
   VkResult (*set_bo_tiling)(struct anv_device *device, struct anv_bo *bo,
                             uint32_t row_pitch_B, enum isl_tiling tiling);
};
