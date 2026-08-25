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
#include "util/detect_os.h"
#include "util/simple_mtx.h"
#include "util/u_idalloc.h"
#if DETECT_OS_LINUX
#include "anv_linux.h"
#endif


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

struct anv_magma_vma {
   uint64_t gpu_va;
   uint64_t size;
};

struct anv_magma_buffer {
   magma_buffer_t buffer;
   void *map_ptr;
   uint64_t map_size;
   uint32_t map_count;
   struct anv_magma_vma *vmas;
   uint32_t vmas_count;
   uint32_t vmas_capacity;
};

struct anv_magma_bind_timeline {
   simple_mtx_t mutex;
   magma_semaphore_t sync;
   uint64_t point;
};

struct anv_magma_device {
   magma_device_t device;
   magma_address_space_t address_space;
   magma_queue_t bind_queue;

   simple_mtx_t bo_mutex;
   struct util_idalloc bo_ids;
   struct anv_magma_buffer *buffers;
   uint32_t buffers_capacity;

   /* Device out-of-band bind timeline semaphore */
   magma_semaphore_t bind_timeline_sync;

   /* Per-queue sparse bind timelines (indexed by queue->bind_queue_id - 1) */
   struct anv_magma_bind_timeline *bind_timelines;
   uint32_t bind_timelines_count;
   uint32_t bind_timelines_capacity;

   /* Per-queue companion RCS queues (indexed by queue->companion_rcs_id - 1) */
   magma_queue_t *companion_rcs_queues;
   uint32_t companion_rcs_queues_count;
   uint32_t companion_rcs_queues_capacity;
};

/* Magma native sync type */
struct anv_magma_sync {
   struct vk_sync base;
   magma_semaphore_t magma_sync;
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
VkResult anv_magma_device_setup_vm(struct anv_device *device,
                                   const VkDeviceCreateInfo *pCreateInfo,
                                   const uint32_t num_queues);
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

/* Buffer table & sync helpers */
magma_buffer_t anv_magma_get_buffer(struct anv_device *device, uint32_t gem_handle);
uint32_t anv_magma_register_buffer(struct anv_device *device, magma_buffer_t buf);
VkResult anv_magma_sync_signal(struct vk_device *device,
                               struct vk_sync *sync,
                               uint64_t value);

#if DETECT_OS_LINUX
VkResult anv_magma_sync_export_sync_file(struct vk_device *device,
                                         struct vk_sync *sync,
                                         int *sync_file);
VkResult anv_magma_sync_import_sync_file(struct vk_device *device,
                                         struct vk_sync *sync,
                                         int sync_file);
void *anv_magma_linux_gem_mmap_placed(struct anv_device *device,
                                      struct anv_bo *bo,
                                      uint64_t offset,
                                      uint64_t size,
                                      void *placed_addr);
void anv_magma_linux_physical_device_init_fd(struct anv_physical_device *device,
                                             const char *primary_path,
                                             const char *path,
                                             int fd);
void anv_magma_linux_physical_device_finish_fd(struct anv_physical_device *device);
extern const struct anv_linux_sys_driver_ops anv_magma_linux_sys_driver_ops;
#endif

/* KMD backend */
const struct anv_kmd_backend *anv_magma_kmd_backend_get(void);


#endif /* ANV_MAGMA_H */
