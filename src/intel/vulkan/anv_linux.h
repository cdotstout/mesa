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
   int (*bo_export_fd)(struct anv_device *device, struct anv_bo *bo);
   uint32_t (*bo_import_fd)(struct anv_device *device, int fd);
   VkResult (*get_bo_tiling)(struct anv_device *device, struct anv_bo *bo,
                             enum isl_tiling *tiling_out);
   VkResult (*set_bo_tiling)(struct anv_device *device, struct anv_bo *bo,
                             uint32_t row_pitch_B, enum isl_tiling tiling);
};

VkResult anv_linux_physical_device_try_create(struct vk_instance *vk_instance,
                                              struct _drmDevice *drm_device,
                                              struct vk_physical_device **out);
VkResult anv_linux_device_map_placed_addr_slab(struct anv_device *device,
                                               struct anv_bo *bo,
                                               uint64_t offset,
                                               size_t size,
                                               void *placed_addr,
                                               void **map_out);
VkResult anv_linux_device_import_bo(struct anv_device *device, int fd,
                                    enum anv_bo_alloc_flags alloc_flags,
                                    uint64_t client_address,
                                    struct anv_bo **bo);
VkResult anv_linux_device_export_bo(struct anv_device *device,
                                    struct anv_bo *bo, int *fd_out);
int anv_gem_bo_export_fd(struct anv_device *device, struct anv_bo *bo);
uint32_t anv_gem_bo_import_fd(struct anv_device *device, int fd);
VkResult anv_linux_bo_unmap(struct anv_device *device, struct anv_bo *bo,
                            void *map, size_t map_size, bool replace);
void anv_linux_physical_device_init_fd(struct anv_physical_device *device,
                                       const char *primary_path,
                                       const char *path,
                                       int fd);
void anv_linux_physical_device_finish_fd(struct anv_physical_device *device);
VkResult anv_linux_device_open(struct anv_device *device,
                               const struct anv_physical_device *physical_device);
void anv_linux_device_close(struct anv_device *device);
bool anv_gem_update_memory_info(struct intel_device_info *devinfo, int fd);
bool anv_gem_read_render_timestamp(int fd, enum intel_kmd_type kmd_type,
                                   uint64_t *timestamp);
bool anv_gem_read_correlate_cpu_gpu_timestamp(int fd,
                                              enum intel_kmd_type kmd_type,
                                              enum intel_engine_class engine_class,
                                              uint16_t engine_instance,
                                              int cpu_clock_id,
                                              uint64_t *cpu_timestamp,
                                              uint64_t *gpu_timestamp,
                                              uint64_t *cpu_delta_timestamp);
