/*
 * Copyright 2026 The Magma GPU Project
 * SPDX-License-Identifier: MIT
 */

#include <assert.h>
#include <errno.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include <magma.h>

#include "anv_private.h"
#include "anv_measure.h"
#include "util/os_file.h"
#include "util/detect_os.h"
#include "magma/anv_magma.h"

/*
 * Buffer handle table helpers.
 * Maps uint32_t gem_handle <-> magma_buffer_t in a thread-safe manner.
 */

static magma_buffer_t
magma_get_buffer(struct anv_device *device, uint32_t gem_handle)
{
   if (!device->magma || gem_handle == 0)
      return NULL;

   simple_mtx_lock(&device->magma->bo_mutex);
   magma_buffer_t buf = NULL;
   if (gem_handle < device->magma->buffers_capacity)
      buf = device->magma->buffers[gem_handle];
   simple_mtx_unlock(&device->magma->bo_mutex);

   return buf;
}

static uint32_t
magma_register_buffer(struct anv_device *device, magma_buffer_t buf)
{
   if (!device->magma)
      return 0;

   simple_mtx_lock(&device->magma->bo_mutex);
   uint32_t id = util_idalloc_alloc(&device->magma->bo_ids) + 1;

   if (id >= device->magma->buffers_capacity) {
      uint32_t new_cap = MAX2(device->magma->buffers_capacity * 2, 64);
      while (new_cap <= id)
         new_cap *= 2;
      device->magma->buffers = vk_realloc(&device->vk.alloc,
                                          device->magma->buffers,
                                          sizeof(magma_buffer_t) * new_cap,
                                          8, VK_SYSTEM_ALLOCATION_SCOPE_DEVICE);
      if (!device->magma->buffers) {
         util_idalloc_free(&device->magma->bo_ids, id - 1);
         device->magma->buffers_capacity = 0;
         simple_mtx_unlock(&device->magma->bo_mutex);
         return 0;
      }
      memset(&device->magma->buffers[device->magma->buffers_capacity], 0,
             sizeof(magma_buffer_t) * (new_cap - device->magma->buffers_capacity));
      device->magma->buffers_capacity = new_cap;
   }

   device->magma->buffers[id] = buf;
   simple_mtx_unlock(&device->magma->bo_mutex);

   return id;
}

static void
magma_unregister_buffer(struct anv_device *device, uint32_t gem_handle)
{
   if (!device->magma || gem_handle == 0)
      return;

   simple_mtx_lock(&device->magma->bo_mutex);
   if (gem_handle < device->magma->buffers_capacity)
      device->magma->buffers[gem_handle] = NULL;
   util_idalloc_free(&device->magma->bo_ids, gem_handle - 1);
   simple_mtx_unlock(&device->magma->bo_mutex);
}

/*
 * Physical device and memory initialization.
 */

static void
anv_magma_set_device_name(struct intel_device_info *devinfo)
{
   char buf[sizeof(devinfo->name)];
   const char *kmd_names[] = { "Xe", "i915" };

   for (unsigned i = 0; i < ARRAY_SIZE(kmd_names); i++) {
      char *pos = strstr(devinfo->name, kmd_names[i]);
      if (pos) {
         size_t prefix_length = pos - devinfo->name;
         size_t name_length = strlen(kmd_names[i]);

         snprintf(buf, sizeof(buf), "%.*sMagma%s",
                  (int)prefix_length, devinfo->name, pos + name_length);
         memcpy(devinfo->name, buf, sizeof(devinfo->name));
         return;
      }
   }
}

VkResult
anv_magma_enumerate_physical_devices(struct vk_instance *vk_instance)
{
   struct anv_instance *instance =
      container_of(vk_instance, struct anv_instance, vk);
   magma_physical_device_t phys_devs[MAGMA_MAX_PHYSICAL_DEVICES];
   uint32_t num_devices = 0;
   magma_status_t status = magma_enumerate_physical_devices(phys_devs, &num_devices);
   if (status != MAGMA_STATUS_SUCCESS || num_devices == 0)
      return VK_ERROR_INCOMPATIBLE_DRIVER;

   uint32_t count = 0;
   for (uint32_t i = 0; i < num_devices; i++) {
      struct magma_physical_device_info info;
      magma_get_physical_device_info(phys_devs[i], &info);

      /* Verify that the vendor is Intel */
      if (info.vendor_id != MAGMA_VENDOR_ID_INTEL &&
          info.vendor_id != (magma_vendor_id_t)0x8086) {
         magma_physical_device_close(&phys_devs[i]);
         continue;
      }

      /* Verify that the device ID is a recognized Intel GPU */
      struct intel_device_info devinfo;
      if (!intel_get_device_info_from_pci_id(info.device_id, &devinfo)) {
         magma_physical_device_close(&phys_devs[i]);
         continue;
      }

      devinfo.kmd_type = INTEL_KMD_TYPE_MAGMA;
      devinfo.pci_domain = info.pci_bus_info.domain;
      devinfo.pci_bus = info.pci_bus_info.bus;
      devinfo.pci_dev = info.pci_bus_info.device;
      devinfo.pci_func = info.pci_bus_info.function;
      devinfo.pci_device_id = info.device_id;
      devinfo.pci_revision_id = info.pci_bus_info.revision_id;
      anv_magma_set_device_name(&devinfo);
      devinfo.has_context_isolation = true;
      devinfo.mem_alignment = devinfo.has_local_mem ? (64 * 1024) : 4096;

      if (devinfo.ver < 9) {
         vk_errorf(instance, VK_ERROR_INCOMPATIBLE_DRIVER,
                   "Vulkan not yet supported on %s", devinfo.name);
         magma_physical_device_close(&phys_devs[i]);
         continue;
      }

      if (devinfo.verx10 == 120)
         BITSET_CLEAR(devinfo.workarounds, INTEL_WA_16013994831);

      if (!devinfo.has_context_isolation) {
         vk_errorf(instance, VK_ERROR_INCOMPATIBLE_DRIVER,
                   "Vulkan requires context isolation for %s", devinfo.name);
         magma_physical_device_close(&phys_devs[i]);
         continue;
      }

      struct vk_physical_device *pdevice = NULL;
      VkResult result = anv_physical_device_create(instance, &devinfo, NULL, NULL, -1, &pdevice);
      if (result != VK_SUCCESS) {
         magma_physical_device_close(&phys_devs[i]);
         continue;
      }

      struct anv_physical_device *anv_device =
         container_of(pdevice, struct anv_physical_device, vk);
      anv_device->magma_physical_device = phys_devs[i];

      anv_magma_physical_device_init_queue_families(anv_device);

      list_addtail(&pdevice->link, &instance->vk.physical_devices.list);
      count++;
   }

   if (count == 0)
      return VK_ERROR_INCOMPATIBLE_DRIVER;

   return VK_SUCCESS;
}

void
anv_magma_physical_device_init_queue_families(struct anv_physical_device *device)
{
   uint32_t family_count = 0;
   VkQueueFlags sparse_flags = device->sparse_type != ANV_SPARSE_TYPE_NOT_SUPPORTED ?
                               VK_QUEUE_SPARSE_BINDING_BIT : 0;
   VkQueueFlags protected_flag = device->has_protected_contexts ?
                                 VK_QUEUE_PROTECTED_BIT : 0;

   struct magma_physical_device_info info = { 0 };
   if (device->magma_physical_device)
      magma_get_physical_device_info(device->magma_physical_device, &info);

   if (info.queue_family_count > 0) {
      for (uint32_t i = 0; i < info.queue_family_count && family_count < ANV_MAX_QUEUE_FAMILIES; i++) {
         struct magma_queue_family_properties *qf = &info.queue_families[i];
         VkQueueFlags vk_flags = 0;
         enum intel_engine_class engine_class = INTEL_ENGINE_CLASS_RENDER;

         if (qf->queue_flags & MAGMA_QUEUE_FLAGS_GRAPHICS) {
            vk_flags |= VK_QUEUE_GRAPHICS_BIT;
            engine_class = INTEL_ENGINE_CLASS_RENDER;
         }
         if (qf->queue_flags & MAGMA_QUEUE_FLAGS_COMPUTE) {
            vk_flags |= VK_QUEUE_COMPUTE_BIT;
            if (!(vk_flags & VK_QUEUE_GRAPHICS_BIT))
               engine_class = INTEL_ENGINE_CLASS_COMPUTE;
         }
         if (qf->queue_flags & MAGMA_QUEUE_FLAGS_TRANSFER) {
            vk_flags |= VK_QUEUE_TRANSFER_BIT;
            if (!(vk_flags & (VK_QUEUE_GRAPHICS_BIT | VK_QUEUE_COMPUTE_BIT)))
               engine_class = INTEL_ENGINE_CLASS_COPY;
         }
         if ((qf->queue_flags & MAGMA_QUEUE_FLAGS_SPARSE_BINDING) && sparse_flags)
            vk_flags |= VK_QUEUE_SPARSE_BINDING_BIT;
         if ((qf->queue_flags & MAGMA_QUEUE_FLAGS_PROTECTED) && protected_flag)
            vk_flags |= VK_QUEUE_PROTECTED_BIT;

         device->queue.families[family_count++] = (struct anv_queue_family) {
            .queueFlags = vk_flags,
            .queueCount = qf->queue_count ? qf->queue_count : 1,
            .engine_class = engine_class,
            .supports_perf = (engine_class == INTEL_ENGINE_CLASS_RENDER),
         };
      }
   }

   if (family_count == 0) {
      /* Default fallback queue families */
      device->queue.families[family_count++] = (struct anv_queue_family) {
         .queueFlags = VK_QUEUE_GRAPHICS_BIT |
                       VK_QUEUE_COMPUTE_BIT |
                       VK_QUEUE_TRANSFER_BIT |
                       sparse_flags |
                       protected_flag,
         .queueCount = 1,
         .engine_class = INTEL_ENGINE_CLASS_RENDER,
         .supports_perf = true,
      };
      device->queue.families[family_count++] = (struct anv_queue_family) {
         .queueFlags = VK_QUEUE_COMPUTE_BIT |
                       VK_QUEUE_TRANSFER_BIT |
                       sparse_flags,
         .queueCount = 1,
         .engine_class = INTEL_ENGINE_CLASS_COMPUTE,
      };
      device->queue.families[family_count++] = (struct anv_queue_family) {
         .queueFlags = VK_QUEUE_TRANSFER_BIT |
                       protected_flag,
         .queueCount = 1,
         .engine_class = INTEL_ENGINE_CLASS_COPY,
      };
   }

   assert(family_count <= ANV_MAX_QUEUE_FAMILIES);
   device->queue.family_count = family_count;

   if (device->vk.wsi_device) {
      device->vk.wsi_device->queue_family_count = family_count;
      for (uint32_t i = 0; i < family_count; i++) {
         if (device->queue.families[i].queueFlags & (VK_QUEUE_GRAPHICS_BIT | VK_QUEUE_COMPUTE_BIT))
            device->vk.wsi_device->queue_supports_blit |= BITFIELD64_BIT(i);
      }
   }
}

VkResult
anv_magma_physical_device_get_parameters(struct anv_physical_device *device)
{
   if (!device->magma_physical_device) {
      magma_physical_device_t phys_devs[MAGMA_MAX_PHYSICAL_DEVICES];
      uint32_t num_devices = 0;
      magma_status_t status = magma_enumerate_physical_devices(phys_devs, &num_devices);
      if (status != MAGMA_STATUS_SUCCESS || num_devices == 0)
         return vk_errorf(device, VK_ERROR_INCOMPATIBLE_DRIVER,
                          "No Magma physical devices found");

      for (uint32_t i = 0; i < num_devices; i++) {
         struct magma_physical_device_info info;
         magma_get_physical_device_info(phys_devs[i], &info);

         /* Verify that the vendor is Intel */
         if (info.vendor_id != MAGMA_VENDOR_ID_INTEL &&
             info.vendor_id != (magma_vendor_id_t)0x8086)
            continue;

         /* Verify that the device ID is a recognized Intel GPU */
         struct intel_device_info devinfo;
         if (!intel_get_device_info_from_pci_id(info.device_id, &devinfo))
            continue;

         /* If physical device already has a PCI ID configured, ensure it matches */
         if (device->info.pci_device_id &&
             device->info.pci_device_id != info.device_id)
            continue;

         device->magma_physical_device = phys_devs[i];
         break;
      }

      if (!device->magma_physical_device)
         return vk_errorf(device, VK_ERROR_INCOMPATIBLE_DRIVER,
                          "No compatible Intel Magma physical device found");
   }

   struct magma_physical_device_info pci_info;
   magma_get_physical_device_info(device->magma_physical_device, &pci_info);
   device->info.pci_domain = pci_info.pci_bus_info.domain;
   device->info.pci_bus = pci_info.pci_bus_info.bus;
   device->info.pci_dev = pci_info.pci_bus_info.device;
   device->info.pci_func = pci_info.pci_bus_info.function;
   device->info.pci_device_id = pci_info.device_id;
   device->info.pci_revision_id = pci_info.pci_bus_info.revision_id;

   device->has_vm_control = true;
   device->max_context_priority = VK_QUEUE_GLOBAL_PRIORITY_MEDIUM;
   device->has_protected_contexts = (device->info.ver >= 12);
   device->sparse_type = ANV_SPARSE_TYPE_VM_BIND;

   /* Query memory properties from Magma */
   struct magma_memory_properties props;
   magma_status_t status = magma_get_memory_properties(device->magma_physical_device, &props);
   if (status == MAGMA_STATUS_SUCCESS && props.memory_heap_count > 0) {
      device->info.mem.sram.mappable.size = props.memory_heaps[0].heap_size;
      device->info.mem.sram.mappable.free = props.memory_heaps[0].heap_size;
   }

   /* Intel Gen8+ GPU virtual address space is 48-bit (256 TiB) */
   device->info.gtt_size = 1ull << 48;
   device->info.mem_alignment = device->info.has_local_mem ? (64 * 1024) : 4096;

   return VK_SUCCESS;
}

VkResult
anv_magma_physical_device_init_memory_types(struct anv_physical_device *device)
{
   if (!device->magma_physical_device)
      return vk_error(device, VK_ERROR_INITIALIZATION_FAILED);

   struct magma_memory_properties props;
   magma_status_t status = magma_get_memory_properties(device->magma_physical_device, &props);
   if (status != MAGMA_STATUS_SUCCESS)
      return vk_error(device, VK_ERROR_INITIALIZATION_FAILED);

   device->memory.heap_count = props.memory_heap_count;
   for (uint32_t i = 0; i < props.memory_heap_count; i++) {
      device->memory.heaps[i] = (struct anv_memory_heap) {
         .size = props.memory_heaps[i].heap_size,
         .flags = props.memory_heaps[i].heap_flags,
         .is_local_mem = (props.memory_heaps[i].heap_flags & VK_MEMORY_HEAP_DEVICE_LOCAL_BIT) != 0,
      };
   }

   device->memory.type_count = props.memory_type_count;
   for (uint32_t i = 0; i < props.memory_type_count; i++) {
      device->memory.types[i] = (struct anv_memory_type) {
         .propertyFlags = props.memory_types[i].property_flags,
         .heapIndex = props.memory_types[i].heap_idx,
      };
   }

   return VK_SUCCESS;
}

/*
 * Device setup and destruction.
 */

/*
 * Native Magma Vulkan Sync Type Implementation.
 */

static inline magma_sync_obj_use_flags_t
vk_sync_to_magma_sync_flags(const struct vk_sync *sync)
{
   magma_sync_obj_use_flags_t flags = 0;

   if (sync->flags & VK_SYNC_IS_SHAREABLE)
      flags |= MAGMA_SYNC_OBJ_USE_FLAGS_EXPORTABLE;

   return flags;
}

static VkResult
anv_magma_sync_init(struct vk_device *device,
                    struct vk_sync *sync,
                    uint64_t initial_value)
{
   struct anv_device *anv_dev = container_of(device, struct anv_device, vk);
   struct anv_magma_sync *msync = anv_magma_sync_as_magma(sync);

   struct magma_create_sync_obj_info info = {
      .header = {
         .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SYNC_OBJ_INFO,
         .size = sizeof(info),
         .p_next = NULL,
      },
      .sync_type = (sync->flags & VK_SYNC_IS_TIMELINE) ?
         MAGMA_SYNC_OBJ_TYPE_TIMELINE : MAGMA_SYNC_OBJ_TYPE_BINARY,
      .use_flags = vk_sync_to_magma_sync_flags(sync),
      .initial_point = initial_value,
   };
   magma_status_t status = magma_create_sync_obj(anv_dev->magma ? anv_dev->magma->device : NULL,
                                                 &info,
                                                 &msync->magma_sync);
   if (status != MAGMA_STATUS_SUCCESS)
      return VK_ERROR_OUT_OF_DEVICE_MEMORY;

   if (!(sync->flags & VK_SYNC_IS_TIMELINE) && initial_value)
      magma_sync_obj_signal(msync->magma_sync);

   return VK_SUCCESS;
}

static void
anv_magma_sync_finish(struct vk_device *device,
                      struct vk_sync *sync)
{
   struct anv_magma_sync *msync = anv_magma_sync_as_magma(sync);
   if (msync->magma_sync) {
      magma_sync_obj_close(&msync->magma_sync);
      msync->magma_sync = NULL;
   }
}

static VkResult
anv_magma_sync_reset(struct vk_device *device,
                     struct vk_sync *sync)
{
   anv_magma_sync_finish(device, sync);
   return anv_magma_sync_init(device, sync, 0);
}

static VkResult
anv_magma_sync_signal(struct vk_device *device,
                      struct vk_sync *sync,
                      uint64_t value)
{
   struct anv_magma_sync *msync = anv_magma_sync_as_magma(sync);

   if (sync->flags & VK_SYNC_IS_TIMELINE) {
      magma_status_t status = magma_sync_obj_timeline_signal(msync->magma_sync,
                                                             value);
      if (status != MAGMA_STATUS_SUCCESS)
         return VK_ERROR_DEVICE_LOST;

      return VK_SUCCESS;
   }

   magma_status_t status = magma_sync_obj_signal(msync->magma_sync);
   if (status != MAGMA_STATUS_SUCCESS)
      return VK_ERROR_DEVICE_LOST;

   return VK_SUCCESS;
}

static VkResult
anv_magma_sync_wait(struct vk_device *device,
                    struct vk_sync *sync,
                    uint64_t wait_value,
                    enum vk_sync_wait_flags wait_flags,
                    uint64_t abs_timeout_ns)
{
   struct anv_magma_sync *msync = anv_magma_sync_as_magma(sync);

   /* DRM syncobj timeouts are signed. OS_TIMEOUT_INFINITE (UINT64_MAX) would
    * be interpreted as a negative absolute deadline and time out instantly.
    */
   abs_timeout_ns = MIN2(abs_timeout_ns, (uint64_t)INT64_MAX);

   if (sync->flags & VK_SYNC_IS_TIMELINE) {
      magma_status_t status = magma_sync_obj_timeline_wait(msync->magma_sync,
                                                           wait_value,
                                                           abs_timeout_ns,
                                                           (uint32_t)wait_flags);
      if (status == MAGMA_STATUS_TIMED_OUT)
         return VK_TIMEOUT;
      if (status != MAGMA_STATUS_SUCCESS)
         return VK_ERROR_DEVICE_LOST;

      return VK_SUCCESS;
   }

   magma_status_t status = magma_sync_obj_wait(msync->magma_sync,
                                               abs_timeout_ns);
   if (status == MAGMA_STATUS_TIMED_OUT)
      return VK_TIMEOUT;
   if (status != MAGMA_STATUS_SUCCESS)
      return VK_ERROR_DEVICE_LOST;

   return VK_SUCCESS;
}

static VkResult
anv_magma_sync_move(struct vk_device *device,
                    struct vk_sync *dst,
                    struct vk_sync *src)
{
   struct anv_magma_sync *dst_msync = anv_magma_sync_as_magma(dst);
   struct anv_magma_sync *src_msync = anv_magma_sync_as_magma(src);

   if (!(dst->flags & VK_SYNC_IS_SHARED) &&
       !(src->flags & VK_SYNC_IS_SHARED)) {
      SWAP(dst_msync->magma_sync, src_msync->magma_sync);
      return VK_SUCCESS;
   } else {
      struct magma_handle handle = { 0 };
      magma_status_t status = magma_sync_obj_export(src_msync->magma_sync, &handle);
      if (status != MAGMA_STATUS_SUCCESS)
         return VK_ERROR_OUT_OF_DEVICE_MEMORY;

      status = magma_sync_obj_import(dst_msync->magma_sync, &handle);
      if (status != MAGMA_STATUS_SUCCESS)
         return VK_ERROR_OUT_OF_DEVICE_MEMORY;

      return VK_SUCCESS;
   }
}

#if DETECT_OS_LINUX
static VkResult
anv_magma_sync_export_sync_file(struct vk_device *device,
                                struct vk_sync *sync,
                                int *sync_file)
{
   struct anv_magma_sync *msync = anv_magma_sync_as_magma(sync);
   struct magma_handle handle = { 0 };

   magma_status_t status = magma_sync_obj_export(msync->magma_sync, &handle);
   if (status != MAGMA_STATUS_SUCCESS)
      return VK_ERROR_OUT_OF_DEVICE_MEMORY;

   *sync_file = (int)handle.os_handle;
   return VK_SUCCESS;
}

static VkResult
anv_magma_sync_import_sync_file(struct vk_device *device,
                                struct vk_sync *sync,
                                int sync_file)
{
   struct anv_magma_sync *msync = anv_magma_sync_as_magma(sync);

   if (sync_file < 0)
      return anv_magma_sync_signal(device, sync, 0);

   /* The Vulkan runtime (vk_common_ImportSemaphoreFdKHR) closes sync_file
    * upon successful import according to the Vulkan specification.
    * Magma takes ownership of the passed handle.
    */
   int dup_handle = os_dupfd_cloexec(sync_file);
   if (dup_handle < 0)
      return vk_error(device, VK_ERROR_OUT_OF_HOST_MEMORY);

   struct magma_handle handle = {
      .os_handle = dup_handle,
      .handle_type = MAGMA_HANDLE_TYPE_SIGNAL_SYNC_FD,
   };
   magma_status_t status = magma_sync_obj_import(msync->magma_sync,
                                                 &handle);
   if (status != MAGMA_STATUS_SUCCESS) {
      close(dup_handle);
      return VK_ERROR_INVALID_EXTERNAL_HANDLE;
   }

   return VK_SUCCESS;
}
#endif

#if DETECT_OS_WINDOWS
static VkResult
anv_magma_sync_export_win32_handle(struct vk_device *device,
                                   struct vk_sync *sync,
                                   void **handle)
{
   struct anv_magma_sync *msync = anv_magma_sync_as_magma(sync);
   struct magma_handle magma_h = { 0 };

   magma_status_t status = magma_sync_obj_export(msync->magma_sync, &magma_h);
   if (status != MAGMA_STATUS_SUCCESS)
      return VK_ERROR_OUT_OF_DEVICE_MEMORY;

   *handle = (void *)(uintptr_t)magma_h.os_handle;
   return VK_SUCCESS;
}

static VkResult
anv_magma_sync_import_win32_handle(struct vk_device *device,
                                   struct vk_sync *sync,
                                   void *handle,
                                   const wchar_t *name)
{
   struct anv_magma_sync *msync = anv_magma_sync_as_magma(sync);

   struct magma_handle magma_h = {
      .os_handle = (int64_t)(uintptr_t)handle,
      .handle_type = 0,
   };
   magma_status_t status = magma_sync_obj_import(msync->magma_sync,
                                                 &magma_h);
   if (status != MAGMA_STATUS_SUCCESS)
      return VK_ERROR_INVALID_EXTERNAL_HANDLE;

   return VK_SUCCESS;
}
#endif

static VkResult
anv_magma_sync_get_value(struct vk_device *device,
                         struct vk_sync *sync,
                         uint64_t *value)
{
   struct anv_magma_sync *msync = anv_magma_sync_as_magma(sync);

   magma_status_t status = magma_sync_obj_timeline_query(msync->magma_sync,
                                                         value);
   if (status != MAGMA_STATUS_SUCCESS)
      return VK_ERROR_DEVICE_LOST;

   return VK_SUCCESS;
}

const struct vk_sync_type anv_magma_sync_type = {
   .size = sizeof(struct anv_magma_sync),
   .features = (enum vk_sync_features)
      (VK_SYNC_FEATURE_BINARY |
       VK_SYNC_FEATURE_GPU_WAIT |
       VK_SYNC_FEATURE_GPU_MULTI_WAIT |
       VK_SYNC_FEATURE_CPU_RESET |
       VK_SYNC_FEATURE_CPU_WAIT |
       VK_SYNC_FEATURE_CPU_SIGNAL |
       VK_SYNC_FEATURE_WAIT_ANY |
       VK_SYNC_FEATURE_WAIT_PENDING),
   .init = anv_magma_sync_init,
   .finish = anv_magma_sync_finish,
   .reset = anv_magma_sync_reset,
   .signal = anv_magma_sync_signal,
   .get_value = anv_magma_sync_get_value,
   .wait = anv_magma_sync_wait,
   .move = anv_magma_sync_move,
#if DETECT_OS_LINUX
   .import_sync_file = anv_magma_sync_import_sync_file,
   .export_sync_file = anv_magma_sync_export_sync_file,
#endif
#if DETECT_OS_WINDOWS
   .import_win32_handle = anv_magma_sync_import_win32_handle,
   .export_win32_handle = anv_magma_sync_export_win32_handle,
#endif
};

void
anv_magma_physical_device_init_sync(struct anv_physical_device *device)
{
   struct magma_sync_properties sync_props = {
      .header = {
         .s_type = MAGMA_STRUCTURE_TYPE_SYNC_PROPERTIES,
         .size = sizeof(sync_props),
         .p_next = NULL,
      },
   };
   magma_get_sync_properties(device->magma_physical_device ? (magma_device_t)device->magma_physical_device : NULL,
                             &sync_props);

   device->sync_syncobj_type = anv_magma_sync_type;
   if (sync_props.capabilities & MAGMA_SYNC_CAPABILITY_TIMELINE)
      device->sync_syncobj_type.features |= VK_SYNC_FEATURE_TIMELINE;

   assert(device->sync_syncobj_type.features & VK_SYNC_FEATURE_CPU_WAIT);

   device->sync_types[0] = &device->sync_syncobj_type;
   device->sync_types[1] = NULL;
   device->vk.supported_sync_types = device->sync_types;
}

VkResult
anv_magma_device_setup_vm(struct anv_device *device)
{
   if (!device->physical->magma_physical_device)
      return vk_error(device, VK_ERROR_INITIALIZATION_FAILED);

   struct anv_magma_device *magma = vk_zalloc(&device->vk.alloc, sizeof(*magma), 8,
                                              VK_SYSTEM_ALLOCATION_SCOPE_DEVICE);
   if (!magma)
      return vk_error(device, VK_ERROR_OUT_OF_HOST_MEMORY);

   simple_mtx_init(&magma->bo_mutex, mtx_plain);
   util_idalloc_init(&magma->bo_ids, 64);
   device->magma = magma;

   magma_status_t status = magma_create_device(device->physical->magma_physical_device,
                                               &magma->device);
   if (status != MAGMA_STATUS_SUCCESS) {
      anv_magma_device_destroy_vm(device);
      return vk_errorf(device, VK_ERROR_INITIALIZATION_FAILED,
                       "magma_create_device failed: %d", status);
   }

   status = magma_create_address_space(magma->device, &magma->address_space);
   if (status != MAGMA_STATUS_SUCCESS) {
      anv_magma_device_destroy_vm(device);
      return vk_errorf(device, VK_ERROR_INITIALIZATION_FAILED,
                       "magma_create_address_space failed: %d", status);
   }

   /*
    * Bind timeline: Asynchronous VM_BIND operations (such as in Linux Xe,
    * WDDM/D3DKMT paging queues, and Magma VM binding) decouple page table
    * updates from execution queues. A bind timeline sync object is used to
    * order out-of-band binds against command buffer submissions.
    */
   struct magma_create_sync_obj_info sync_info = {
      .header = {
         .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SYNC_OBJ_INFO,
         .size = sizeof(sync_info),
         .p_next = NULL,
      },
      .sync_type = MAGMA_SYNC_OBJ_TYPE_TIMELINE,
      .use_flags = 0,
      .initial_point = 0,
   };
   status = magma_create_sync_obj(magma->device, &sync_info, &magma->bind_timeline_sync);
   if (status != MAGMA_STATUS_SUCCESS) {
      anv_magma_device_destroy_vm(device);
      return vk_errorf(device, VK_ERROR_INITIALIZATION_FAILED,
                       "magma_create_sync_obj failed: %d", status);
   }

   simple_mtx_init(&device->bind_timeline.mutex, mtx_plain);
   device->bind_timeline.point = 0;
   device->bind_timeline.syncobj = 0;

   if (device->fd >= 0) {
      if (!intel_bind_timeline_init(&device->bind_timeline, device->fd)) {
         anv_magma_device_destroy_vm(device);
         return vk_errorf(device, VK_ERROR_INITIALIZATION_FAILED,
                          "intel_bind_timeline_init failed");
      }
   }

   return VK_SUCCESS;
}

bool
anv_magma_device_destroy_vm(struct anv_device *device)
{
   if (!device->magma)
      return true;

   if (device->magma->bind_timeline_sync) {
      uint64_t point = intel_bind_timeline_get_last_point(&device->bind_timeline);
      if (point > 0) {
         magma_sync_obj_timeline_wait(device->magma->bind_timeline_sync,
                                      point,
                                      (uint64_t)INT64_MAX,
                                      0);
      }
      magma_sync_obj_close(&device->magma->bind_timeline_sync);
   }

   if (device->fd >= 0 && device->bind_timeline.syncobj != 0) {
      intel_bind_timeline_finish(&device->bind_timeline, device->fd);
   } else {
      simple_mtx_destroy(&device->bind_timeline.mutex);
   }

   if (device->magma->address_space)
      magma_address_space_close(&device->magma->address_space);

   if (device->magma->device) {
      magma_device_close(&device->magma->device);
   }

   simple_mtx_destroy(&device->magma->bo_mutex);
   util_idalloc_fini(&device->magma->bo_ids);
   vk_free(&device->vk.alloc, device->magma->buffers);
   vk_free(&device->vk.alloc, device->magma);
   device->magma = NULL;

   return true;
}

VkResult
anv_magma_device_check_status(struct vk_device *vk_device)
{
   struct anv_device *device = container_of(vk_device, struct anv_device, vk);
   VkResult result = VK_SUCCESS;

   for (uint32_t i = 0; i < device->queue_count; i++) {
      struct anv_queue *queue = &device->queues[i];
      if (queue->magma_queue) {
         magma_status_t status = magma_queue_check_status(queue->magma_queue);
         if (status == MAGMA_STATUS_CONTEXT_KILLED) {
            result = anv_queue_set_lost(queue, ECANCELED, "Queue context killed/reset");
            goto done;
         } else if (status != MAGMA_STATUS_SUCCESS) {
            result = anv_queue_set_lost(queue, EIO, "magma_queue_check_status failed: %d", status);
            goto done;
         }
      }
      if (queue->magma_bind_queue) {
         magma_status_t status = magma_queue_check_status(queue->magma_bind_queue);
         if (status == MAGMA_STATUS_CONTEXT_KILLED) {
            result = anv_queue_set_lost(queue, ECANCELED, "Bind queue context killed/reset");
            goto done;
         } else if (status != MAGMA_STATUS_SUCCESS) {
            result = anv_queue_set_lost(queue, EIO, "magma_queue_check_status failed on bind queue: %d", status);
            goto done;
         }
      }
   }

done:
   if (anv_needs_printf_buffer()) {
      VkResult print_result = vk_check_printf_status(vk_device, &device->printf);
      result = result != VK_SUCCESS ? result : print_result;
   }

   return result;
}

struct intel_pagefault_buffer *
anv_magma_device_alloc_get_vm_faults(struct anv_device *device)
{
   /*
    * TODO(magma): Magma currently does not have an FFI endpoint to retrieve GPU VM page faults.
    *
    * How it should be implemented:
    * 1. Add `magma_device_get_vm_faults` to Gorgonzola command specs.
    * 2. In the driver backend, read the page fault buffer or error state.
    * 3. Populate and return `struct intel_pagefault_buffer`.
    */
   return NULL;
}

/*
 * Queue management.
 */

VkResult
anv_magma_create_engine(struct anv_device *device,
                        struct anv_queue *queue,
                        const VkDeviceQueueCreateInfo *pCreateInfo)
{
   if (!device->magma || !device->magma->device || !device->magma->address_space)
      return VK_SUCCESS;

   uint32_t flags = 0;
   if (pCreateInfo->flags & VK_DEVICE_QUEUE_CREATE_PROTECTED_BIT)
      flags |= MAGMA_QUEUE_FLAGS_PROTECTED;

   struct magma_create_queue_info info = {
      .queue_family_idx = pCreateInfo->queueFamilyIndex,
      .priority = pCreateInfo->pQueuePriorities ? (uint32_t)(pCreateInfo->pQueuePriorities[0] * 100) : 0,
      .flags = flags,
   };

   magma_status_t status = magma_create_queue(device->magma->device,
                                              device->magma->address_space,
                                              &info,
                                              &queue->magma_queue);
   if (status != MAGMA_STATUS_SUCCESS)
      return vk_error(device, VK_ERROR_INITIALIZATION_FAILED);

   if (queue->family && (queue->family->queueFlags & VK_QUEUE_SPARSE_BINDING_BIT)) {
      struct magma_create_queue_info bind_info = {
         .queue_family_idx = pCreateInfo->queueFamilyIndex,
         .priority = info.priority,
         .flags = flags | MAGMA_QUEUE_FLAGS_SPARSE_BINDING,
      };
      status = magma_create_queue(device->magma->device,
                                  device->magma->address_space,
                                  &bind_info,
                                  &queue->magma_bind_queue);
      if (status != MAGMA_STATUS_SUCCESS) {
         magma_queue_close(&queue->magma_queue);
         return vk_errorf(device, VK_ERROR_INITIALIZATION_FAILED,
                          "magma_create_queue for sparse bind queue failed: %d", status);
      }
   }

   return VK_SUCCESS;
}

void
anv_magma_destroy_engine(struct anv_device *device, struct anv_queue *queue)
{
   if (queue->magma_bind_queue)
      magma_queue_close(&queue->magma_bind_queue);
   if (queue->magma_queue)
      magma_queue_close(&queue->magma_queue);
}

/*
 * Batch submission.
 */

static void
anv_magma_init_submit_info(struct magma_submit_info *submit_info,
                           struct magma_submit_address_space_info *addr_info,
                           struct anv_device *device,
                           uint64_t command_va,
                           uint64_t length)
{
   memset(submit_info, 0, sizeof(*submit_info));
   memset(addr_info, 0, sizeof(*addr_info));

   addr_info->header = (struct magma_structure_type_header) {
      .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_ADDRESS_SPACE_INFO,
      .size = sizeof(*addr_info),
      .p_next = NULL,
   };
   addr_info->command_va = command_va;
   addr_info->length = length;

   submit_info->header = (struct magma_structure_type_header) {
      .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_INFO,
      .size = sizeof(*submit_info),
      .p_next = addr_info,
   };
   submit_info->flags = 0;
   submit_info->sync_info.header = (struct magma_structure_type_header) {
      .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO,
      .size = sizeof(submit_info->sync_info),
      .p_next = NULL,
   };
}

static void
anv_magma_submit_add_sync(struct magma_submit_sync_info *sync_info,
                          struct vk_sync *sync,
                          uint64_t point,
                          bool is_signal)
{
   if (!sync)
      return;

   struct anv_magma_sync *msync = anv_magma_sync_as_magma(sync);
   if (!msync || !msync->magma_sync)
      return;

   if (is_signal) {
      if (sync_info->num_signal_sync_objs < MAGMA_MAX_SYNCOBJS) {
         uint32_t idx = sync_info->num_signal_sync_objs++;
         sync_info->signal_sync_objs[idx] = msync->magma_sync;
         sync_info->signal_points[idx] = point;
      }
   } else {
      if (sync_info->num_wait_sync_objs < MAGMA_MAX_SYNCOBJS) {
         uint32_t idx = sync_info->num_wait_sync_objs++;
         sync_info->wait_sync_objs[idx] = msync->magma_sync;
         sync_info->wait_points[idx] = point;
      }
   }
}

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
                            struct anv_utrace_submit *utrace_submit)
{
   struct anv_device *device = queue->device;

   uint64_t command_va = 0;
   uint64_t length = 0;
   if (cmd_buffer_count) {
      if (unlikely(device->physical->measure_device.config)) {
         for (uint32_t i = 0; i < cmd_buffer_count; i++)
            anv_measure_submit(cmd_buffers[i]);
      }

      anv_cmd_buffer_chain_command_buffers(cmd_buffers, cmd_buffer_count);

#ifdef SUPPORT_INTEL_INTEGRATED_GPUS
      if (device->physical->memory.need_flush &&
          anv_bo_needs_host_cache_flush(device->batch_bo_pool.bo_alloc_flags))
         anv_cmd_buffer_clflush(cmd_buffers, cmd_buffer_count);
#endif

      struct anv_cmd_buffer *first_cmd_buffer = cmd_buffers[0];
      struct anv_batch_bo *first_batch_bo = list_first_entry(&first_cmd_buffer->batch_bos,
                                                             struct anv_batch_bo, link);
      command_va = first_batch_bo->bo->offset;
      length = first_batch_bo->length;
   } else {
      command_va = device->trivial_batch_bo ? device->trivial_batch_bo->offset : 0;
      length = device->trivial_batch_bo ? device->trivial_batch_bo->size : 0;
   }

   if (INTEL_DEBUG(DEBUG_SUBMIT)) {
      fprintf(stderr, "Magma batch offset=0x%016"PRIx64" on queue %u\n",
              command_va, queue->vk.index_in_family);
   }

   /*
    * Construct the Magma command submission structure.
    * Magma is OS-agnostic and passes the address space and command buffer VA directly.
    */
   struct magma_submit_info submit_info;
   struct magma_submit_address_space_info addr_info;
   anv_magma_init_submit_info(&submit_info, &addr_info, device, command_va, length);

   for (uint32_t i = 0; i < wait_count; i++)
      anv_magma_submit_add_sync(&submit_info.sync_info, waits[i].sync, waits[i].wait_value, false);

   for (uint32_t i = 0; i < signal_count; i++)
      anv_magma_submit_add_sync(&submit_info.sync_info, signals[i].sync, signals[i].signal_value, true);

   if (queue->sync)
      anv_magma_submit_add_sync(&submit_info.sync_info, queue->sync, 0, true);

   if (queue->magma_queue) {
      magma_status_t status = magma_submit_command(queue->magma_queue, &submit_info);
      if (status != MAGMA_STATUS_SUCCESS)
         return anv_queue_set_lost(queue, EIO, "magma_submit_command failed: %d", status);
   }

   return VK_SUCCESS;
}

VkResult
anv_magma_queue_exec_async(struct anv_async_submit *submit,
                           uint32_t wait_count,
                           const struct vk_sync_wait *waits,
                           uint32_t signal_count,
                           const struct vk_sync_signal *signals)
{
   struct anv_queue *queue = submit->queue;
   struct anv_device *device = queue->device;

#ifdef SUPPORT_INTEL_INTEGRATED_GPUS
   if (device->physical->memory.need_flush &&
       anv_bo_needs_host_cache_flush(device->utrace_bo_pool.bo_alloc_flags)) {
      util_dynarray_foreach(&submit->batch_bos, struct anv_bo *, bo) {
         if ((*bo)->map)
            util_flush_range((*bo)->map, (*bo)->size);
      }
   }
#endif

   struct anv_bo *batch_bo = *util_dynarray_element(&submit->batch_bos, struct anv_bo *, 0);
   struct magma_submit_info submit_info;
   struct magma_submit_address_space_info addr_info;
   anv_magma_init_submit_info(&submit_info, &addr_info, device, batch_bo->offset, batch_bo->size);

   for (uint32_t i = 0; i < wait_count; i++)
      anv_magma_submit_add_sync(&submit_info.sync_info, waits[i].sync, waits[i].wait_value, false);

   for (uint32_t i = 0; i < signal_count; i++)
      anv_magma_submit_add_sync(&submit_info.sync_info, signals[i].sync, signals[i].signal_value, true);

   if (submit->signal.sync)
      anv_magma_submit_add_sync(&submit_info.sync_info, submit->signal.sync, submit->signal.signal_value, true);

   if (queue->sync)
      anv_magma_submit_add_sync(&submit_info.sync_info, queue->sync, 0, true);

   if (queue->magma_queue) {
      magma_status_t status = magma_submit_command(queue->magma_queue, &submit_info);
      if (status != MAGMA_STATUS_SUCCESS)
         return anv_queue_set_lost(queue, EIO, "magma_submit_command async failed: %d", status);
   }

   return anv_queue_post_submit(queue, VK_SUCCESS);
}

/*
 * KMD Backend Implementation.
 */

static void
anv_magma_init_create_buffer_info(struct magma_create_buffer_info *info,
                                  struct anv_device *device,
                                  uint64_t aligned_size,
                                  uint32_t memory_type_idx,
                                  uint32_t common_flags)
{
   memset(info, 0, sizeof(*info));
   info->header = (struct magma_structure_type_header) {
      .s_type = MAGMA_STRUCTURE_TYPE_CREATE_BUFFER_INFO,
      .size = sizeof(*info),
   };
   info->memory_type_idx = memory_type_idx;
   info->alignment = device->info->mem_alignment;
   info->common_flags = common_flags;
   info->vendor_flags = 0;
   info->size = aligned_size;
}

static uint32_t
anv_magma_choose_memory_type(struct anv_device *device,
                             enum anv_bo_alloc_flags alloc_flags)
{
   const struct anv_physical_device *pdev = device->physical;
   if (pdev->memory.type_count == 0)
      return 0;

   VkMemoryPropertyFlags req = 0;
   if (alloc_flags & ANV_BO_ALLOC_PROTECTED)
      req |= VK_MEMORY_PROPERTY_PROTECTED_BIT;

   if (alloc_flags & (ANV_BO_ALLOC_MAPPED | ANV_BO_ALLOC_LOCAL_MEM_CPU_VISIBLE))
      req |= VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT;

   if ((alloc_flags & ANV_BO_ALLOC_HOST_CACHED_COHERENT) == ANV_BO_ALLOC_HOST_CACHED_COHERENT)
      req |= VK_MEMORY_PROPERTY_HOST_CACHED_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT;
   else if (alloc_flags & ANV_BO_ALLOC_HOST_COHERENT)
      req |= VK_MEMORY_PROPERTY_HOST_COHERENT_BIT;

   bool avoid_local_mem = (alloc_flags & ANV_BO_ALLOC_NO_LOCAL_MEM) != 0;

   /* Try to find match respecting avoid_local_mem */
   for (uint32_t i = 0; i < pdev->memory.type_count; i++) {
      VkMemoryPropertyFlags flags = pdev->memory.types[i].propertyFlags;
      if ((flags & req) != req)
         continue;

      bool is_local = (flags & VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT) != 0;
      if (avoid_local_mem && is_local)
         continue;

      return i;
   }

   /* Fallback: match required properties regardless of local_mem */
   for (uint32_t i = 0; i < pdev->memory.type_count; i++) {
      VkMemoryPropertyFlags flags = pdev->memory.types[i].propertyFlags;
      if ((flags & req) == req)
         return i;
   }

   return 0;
}

static uint32_t
magma_gem_create(struct anv_device *device,
                 const struct intel_memory_class_instance **regions,
                 uint16_t regions_count, uint64_t size,
                 enum anv_bo_alloc_flags alloc_flags,
                 uint64_t *actual_size)
{
   uint64_t aligned_size = align64(size, device->info->mem_alignment);

   uint32_t common_flags = 0;
   if (alloc_flags & ANV_BO_ALLOC_EXTERNAL)
      common_flags |= MAGMA_BUFFER_FLAG_EXTERNAL;
   if (alloc_flags & ANV_BO_ALLOC_SCANOUT)
      common_flags |= MAGMA_BUFFER_FLAG_SCANOUT;

   uint32_t memory_type_idx = anv_magma_choose_memory_type(device, alloc_flags);

   struct magma_create_buffer_info info;
   anv_magma_init_create_buffer_info(&info, device, aligned_size, memory_type_idx, common_flags);

   magma_buffer_t magma_buf = NULL;
   if (device->magma && device->magma->device) {
      magma_status_t status = magma_create_buffer(device->magma->device, &info, &magma_buf);
      if (status != MAGMA_STATUS_SUCCESS)
         return 0;
   }

   uint32_t gem_handle = magma_register_buffer(device, magma_buf);
   if (!gem_handle)
      return 0;

   *actual_size = aligned_size;
   return gem_handle;
}

static uint32_t
magma_gem_create_userptr(struct anv_device *device, void *mem, uint64_t size)
{
   /*
    * USERPTR is deliberately unsupported in the clean, OS-agnostic Magma API.
    * Applications importing host pointers via VK_EXT_external_memory_host will
    * cleanly receive VK_ERROR_INVALID_EXTERNAL_HANDLE.
    */
   return 0;
}

static void
magma_gem_close(struct anv_device *device, struct anv_bo *bo)
{
   if (bo->from_host_ptr || !bo->gem_handle)
      return;

   magma_buffer_t magma_buf = magma_get_buffer(device, bo->gem_handle);
   magma_unregister_buffer(device, bo->gem_handle);
   magma_buffer_close(&magma_buf);
}

#ifndef MAP_FAILED
#define MAP_FAILED ((void *)(intptr_t)-1)
#endif

static void *
magma_gem_mmap(struct anv_device *device, struct anv_bo *bo, uint64_t offset,
               uint64_t size, void *placed_addr)
{
   magma_buffer_t magma_buf = magma_get_buffer(device, bo->gem_handle);
   if (!magma_buf)
      return MAP_FAILED;

   struct magma_mapping mapping = { 0 };
   if (magma_map_buffer_cpu(magma_buf, &mapping) == MAGMA_STATUS_SUCCESS && mapping.ptr)
      return (void *)((uintptr_t)mapping.ptr + offset);

   return MAP_FAILED;
}

static void
magma_gem_munmap(struct anv_device *device, struct anv_bo *bo,
                 void *map, size_t map_size)
{
   magma_buffer_t magma_buf = magma_get_buffer(device, bo->gem_handle);
   if (!magma_buf)
      return;

   magma_unmap_buffer_cpu(magma_buf);
}

static int
magma_gem_handle_to_fd(struct anv_device *device, uint32_t gem_handle)
{
   if (!device->magma || gem_handle == 0)
      return -EINVAL;

   magma_buffer_t magma_buf = magma_get_buffer(device, gem_handle);
   if (!magma_buf)
      return -ENOENT;

   struct magma_handle handle = { 0 };
   magma_status_t status = magma_buffer_export(magma_buf, &handle);
   if (status != MAGMA_STATUS_SUCCESS || handle.os_handle < 0)
      return -ENOMEM;

   return (int)handle.os_handle;
}

static uint32_t
magma_gem_fd_to_handle(struct anv_device *device, int fd)
{
   if (!device->magma || !device->magma->device || fd < 0)
      return 0;

   magma_buffer_t magma_buf = NULL;
   struct magma_handle handle = {
      .os_handle = fd,
      .handle_type = 0,
   };
   magma_status_t status = magma_buffer_import(device->magma->device,
                                               &handle,
                                               &magma_buf);
   if (status != MAGMA_STATUS_SUCCESS || !magma_buf)
      return 0;

   return magma_register_buffer(device, magma_buf);
}

static VkResult
magma_vm_bind(struct anv_device *device, struct anv_sparse_submission *submit,
              enum anv_vm_bind_flags flags)
{
   if (!device->magma || !device->magma->address_space)
      return VK_SUCCESS;

   for (uint32_t i = 0; i < submit->binds_len; i++) {
      struct anv_vm_bind *bind = &submit->binds[i];
      if (bind->op == ANV_VM_BIND) {
         magma_buffer_t magma_buf = bind->bo ? magma_get_buffer(device, bind->bo->gem_handle) : NULL;
         if (magma_buf) {
            uint64_t map_flags = MAGMA_GPU_MAP_FLAGS_READ | MAGMA_GPU_MAP_FLAGS_WRITE |
                                 MAGMA_GPU_MAP_FLAGS_EXECUTE;
            magma_status_t status = magma_map_buffer_gpu(device->magma->address_space,
                                                         magma_buf,
                                                         bind->bo_offset,
                                                         intel_48b_address(bind->address),
                                                         bind->size,
                                                         map_flags);
            if (status != MAGMA_STATUS_SUCCESS)
               return vk_errorf(device, VK_ERROR_OUT_OF_DEVICE_MEMORY,
                                "magma_map_buffer_gpu failed: %d", status);
         }
      } else if (bind->op == ANV_VM_UNBIND) {
         magma_status_t status = magma_unmap_buffer_gpu(device->magma->address_space,
                                                        intel_48b_address(bind->address),
                                                        bind->size);
         if (status != MAGMA_STATUS_SUCCESS)
            return vk_errorf(device, VK_ERROR_UNKNOWN,
                             "magma_unmap_buffer_gpu failed: %d", status);
      } else if (bind->op == ANV_VM_UNBIND_ALL) {
         if (bind->bo) {
            magma_status_t status = magma_unmap_buffer_gpu(device->magma->address_space,
                                                           intel_48b_address(bind->bo->offset),
                                                           bind->bo->actual_size);
            if (status != MAGMA_STATUS_SUCCESS)
               return vk_errorf(device, VK_ERROR_UNKNOWN,
                                "magma_unmap_buffer_gpu failed: %d", status);
         }
      }
   }

   /*
    * If a queue is provided and there are syncs (from sparse binding), submit
    * them to the sparse bind queue on GPU without stalling the CPU.
    */
   magma_queue_t bind_queue = submit->queue ?
      (submit->queue->magma_bind_queue ? submit->queue->magma_bind_queue : submit->queue->magma_queue) : NULL;

   if (bind_queue && (submit->wait_count > 0 || submit->signal_count > 0)) {
      struct magma_submit_info submit_info;
      struct magma_submit_address_space_info addr_info;
      anv_magma_init_submit_info(&submit_info, &addr_info, device, 0, 0);

      for (uint32_t i = 0; i < submit->wait_count; i++)
         anv_magma_submit_add_sync(&submit_info.sync_info, submit->waits[i].sync, submit->waits[i].wait_value, false);

      for (uint32_t i = 0; i < submit->signal_count; i++)
         anv_magma_submit_add_sync(&submit_info.sync_info, submit->signals[i].sync, submit->signals[i].signal_value, true);

      magma_status_t status = magma_submit_command(bind_queue, &submit_info);
      if (status != MAGMA_STATUS_SUCCESS)
         return anv_queue_set_lost(submit->queue, EIO, "magma_submit_command on bind queue failed: %d", status);
   }

   if (flags & ANV_VM_BIND_FLAG_SIGNAL_BIND_TIMELINE) {
      uint64_t point = intel_bind_timeline_bind_begin(&device->bind_timeline);
      if (device->magma && device->magma->bind_timeline_sync) {
         magma_sync_obj_timeline_signal(device->magma->bind_timeline_sync,
                                        point);
      }
      intel_bind_timeline_bind_end(&device->bind_timeline);
   }

   ANV_RMV(vm_binds, device, submit->binds, submit->binds_len);

   return VK_SUCCESS;
}

static VkResult
magma_vm_bind_bo(struct anv_device *device, struct anv_bo *bo)
{
   struct anv_vm_bind bind = {
      .bo = bo,
      .address = bo->offset,
      .bo_offset = 0,
      .size = bo->actual_size,
      .op = ANV_VM_BIND,
   };
   struct anv_sparse_submission submit = {
      .queue = NULL,
      .binds = &bind,
      .binds_len = 1,
      .binds_capacity = 1,
      .wait_count = 0,
      .signal_count = 0,
   };
   return magma_vm_bind(device, &submit, ANV_VM_BIND_FLAG_SIGNAL_BIND_TIMELINE);
}

static VkResult
magma_vm_unbind_bo(struct anv_device *device, struct anv_bo *bo)
{
   struct anv_vm_bind bind;
   if (bo->alloc_flags & ANV_BO_ALLOC_NULL_INITIALIZED_HEAP) {
      bind = (struct anv_vm_bind) {
         .address = bo->offset,
         .size = bo->actual_size,
         .op = ANV_VM_BIND,
      };
   } else if (bo->from_host_ptr) {
      bind = (struct anv_vm_bind) {
         .bo = bo,
         .address = bo->offset,
         .size = bo->actual_size,
         .op = ANV_VM_UNBIND,
      };
   } else {
      bind = (struct anv_vm_bind) {
         .bo = bo,
         .op = ANV_VM_UNBIND_ALL,
      };
   }
   struct anv_sparse_submission submit = {
      .queue = NULL,
      .binds = &bind,
      .binds_len = 1,
      .binds_capacity = 1,
      .wait_count = 0,
      .signal_count = 0,
   };
   return magma_vm_bind(device, &submit, ANV_VM_BIND_FLAG_SIGNAL_BIND_TIMELINE);
}

static uint32_t
magma_bo_alloc_flags_to_bo_flags(struct anv_device *device,
                                 enum anv_bo_alloc_flags alloc_flags)
{
   return 0;
}

const struct anv_kmd_backend *
anv_magma_kmd_backend_get(void)
{
   static const struct anv_kmd_backend magma_backend = {
      .gem_create = magma_gem_create,
      .gem_create_userptr = magma_gem_create_userptr,
      .gem_close = magma_gem_close,
      .gem_mmap = magma_gem_mmap,
      .gem_munmap = magma_gem_munmap,
      .vm_bind = magma_vm_bind,
      .vm_bind_bo = magma_vm_bind_bo,
      .vm_unbind_bo = magma_vm_unbind_bo,
      .queue_exec_locked = anv_magma_queue_exec_locked,
      .queue_exec_async = anv_magma_queue_exec_async,
      .bo_alloc_flags_to_bo_flags = magma_bo_alloc_flags_to_bo_flags,
      .gem_handle_to_fd = magma_gem_handle_to_fd,
      .gem_fd_to_handle = magma_gem_fd_to_handle,
   };
   return &magma_backend;
}
