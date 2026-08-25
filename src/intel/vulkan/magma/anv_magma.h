/*
 * Copyright 2026 The Magma GPU Project
 * SPDX-License-Identifier: MIT
 */

#ifndef ANV_MAGMA_H
#define ANV_MAGMA_H

#include <stdbool.h>
#include <stdint.h>
#include <magma.h>

#include "vulkan/vulkan_core.h"
#include "vk_device.h"
#include "vk_sync.h"
#include "util/simple_mtx.h"
#include "util/u_idalloc.h"


struct vk_instance;
struct anv_device;
struct anv_physical_device;
struct anv_queue;
struct anv_bo;
struct anv_cmd_buffer;
struct anv_query_pool;
struct anv_async_submit;
struct anv_utrace_submit;
struct intel_pagefault_buffer;
struct util_sync_provider;

struct anv_magma_device {
   magma_device_t device;
   magma_address_space_t address_space;

   simple_mtx_t bo_mutex;
   struct util_idalloc bo_ids;
   magma_buffer_t *buffers;
   uint32_t buffers_capacity;

   /* Bind timeline sync object */
   magma_sync_obj_t bind_timeline_sync;
};

/* Magma native sync type */
struct anv_magma_sync {
   struct vk_sync base;
   magma_sync_obj_t magma_sync;
};

extern const struct vk_sync_type anv_magma_sync_type;

static inline struct anv_magma_sync *
anv_magma_sync_as_magma(struct vk_sync *sync)
{
   if (!sync)
      return NULL;
   return (struct anv_magma_sync *)sync;
}

/* Device & VM management */
bool anv_magma_device_destroy_vm(struct anv_device *device);
VkResult anv_magma_device_setup_vm(struct anv_device *device);
VkResult anv_magma_device_check_status(struct vk_device *vk_device);
struct intel_pagefault_buffer *
anv_magma_device_alloc_get_vm_faults(struct anv_device *device);
VkResult
anv_magma_enumerate_physical_devices(struct vk_instance *vk_instance);
VkResult
anv_magma_physical_device_get_parameters(struct anv_physical_device *device);
VkResult
anv_magma_physical_device_init_memory_types(struct anv_physical_device *device);
void
anv_magma_physical_device_init_sync(struct anv_physical_device *device);
void
anv_magma_physical_device_init_queue_families(struct anv_physical_device *device);

/* Queue management */
VkResult
anv_magma_create_engine(struct anv_device *device,
                        struct anv_queue *queue,
                        const VkDeviceQueueCreateInfo *pCreateInfo);
void
anv_magma_destroy_engine(struct anv_device *device, struct anv_queue *queue);

/* Batch submission */
VkResult
anv_magma_queue_exec_locked(struct anv_queue *queue,
                            uint32_t wait_count,
                            const struct vk_sync_wait *waits,
                            uint32_t cmd_buffer_count,
                            struct anv_cmd_buffer **cmd_buffers,
                            uint32_t signal_count,
                            const struct vk_sync_signal *signals,
                            struct anv_query_pool *perf_query_pool,
                            uint32_t perf_query_pass,
                            struct anv_utrace_submit *utrace_submit);

VkResult
anv_magma_queue_exec_async(struct anv_async_submit *submit,
                           uint32_t wait_count,
                           const struct vk_sync_wait *waits,
                           uint32_t signal_count,
                           const struct vk_sync_signal *signals);

/* KMD backend */
const struct anv_kmd_backend *anv_magma_kmd_backend_get(void);


#endif /* ANV_MAGMA_H */
