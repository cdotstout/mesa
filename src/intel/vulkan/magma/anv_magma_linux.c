/*
 * Copyright 2026 The Magma GPU Project
 * SPDX-License-Identifier: MIT
 */

#include <errno.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#ifdef MAJOR_IN_SYSMACROS
#include <sys/sysmacros.h>
#endif
#include <unistd.h>
#include <xf86drm.h>

#include "anv_private.h"
#include "anv_linux.h"
#include "util/os_file.h"
#include "util/os_mman.h"
#include "magma/anv_magma.h"

VkResult
anv_magma_sync_export_sync_file(struct vk_device *device,
                                struct vk_sync *sync,
                                int *sync_file)
{
   struct anv_magma_sync *msync = anv_magma_sync_as_magma(sync);
   struct magma_handle handle = { 0 };

   magma_status_t status = magma_semaphore_export(msync->magma_sync, &handle);
   if (status != MAGMA_STATUS_SUCCESS)
      return VK_ERROR_OUT_OF_DEVICE_MEMORY;

   *sync_file = (int)handle.os_handle;
   return VK_SUCCESS;
}

VkResult
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
   magma_status_t status = magma_semaphore_import(msync->magma_sync,
                                                  &handle);
   if (status != MAGMA_STATUS_SUCCESS)
      return VK_ERROR_INVALID_EXTERNAL_HANDLE;

   return VK_SUCCESS;
}

static int
anv_magma_bo_export_fd(struct anv_device *device, struct anv_bo *bo)
{
   if (!device->magma || bo->gem_handle == 0)
      return -EINVAL;

   magma_buffer_t magma_buf = anv_magma_get_buffer(device, bo->gem_handle);
   if (!magma_buf)
      return -ENOENT;

   struct magma_handle handle = { 0 };
   magma_status_t status = magma_buffer_export(magma_buf, &handle);
   if (status != MAGMA_STATUS_SUCCESS || handle.os_handle < 0)
      return -ENOMEM;

   return (int)handle.os_handle;
}

static uint32_t
anv_magma_bo_import_fd(struct anv_device *device, int fd)
{
   if (!device->magma || !device->magma->device || fd < 0)
      return 0;

   int dup_fd = os_dupfd_cloexec(fd);
   if (dup_fd < 0)
      return 0;

   magma_buffer_t magma_buf = NULL;
   struct magma_handle handle = {
      .os_handle = dup_fd,
      .handle_type = MAGMA_HANDLE_TYPE_MEM_DMABUF,
   };
   magma_status_t status = magma_device_import_buffer(device->magma->device,
                                                      &handle,
                                                      &magma_buf);
   if (status != MAGMA_STATUS_SUCCESS || !magma_buf)
      return 0;

   return anv_magma_register_buffer(device, magma_buf);
}

void *
anv_magma_linux_gem_mmap_placed(struct anv_device *device, struct anv_bo *bo,
                                uint64_t offset, uint64_t size,
                                void *placed_addr)
{
   int prime_fd = anv_magma_bo_export_fd(device, bo);
   if (prime_fd < 0)
      return MAP_FAILED;

   void *map = os_mmap(placed_addr, size, PROT_READ | PROT_WRITE,
                       MAP_FIXED | MAP_SHARED, prime_fd, offset);
   close(prime_fd);
   return map;
}

static VkResult
anv_magma_get_bo_tiling(struct anv_device *device, struct anv_bo *bo,
                        enum isl_tiling *tiling_out)
{
   *tiling_out = ISL_TILING_LINEAR;
   return VK_SUCCESS;
}

static VkResult
anv_magma_set_bo_tiling(struct anv_device *device, struct anv_bo *bo,
                        uint32_t row_pitch_B, enum isl_tiling tiling)
{
   return VK_SUCCESS;
}

static int
anv_magma_perf_open(struct anv_device *device, struct anv_queue *queue,
                    uint64_t metric_id)
{
   return -1;
}

static void
anv_magma_perf_close(struct anv_device *device)
{
}

const struct anv_linux_sys_driver_ops anv_magma_linux_sys_driver_ops = {
   .bo_export_fd = anv_magma_bo_export_fd,
   .bo_import_fd = anv_magma_bo_import_fd,
   .get_bo_tiling = anv_magma_get_bo_tiling,
   .set_bo_tiling = anv_magma_set_bo_tiling,
   .perf_open = anv_magma_perf_open,
   .perf_close = anv_magma_perf_close,
};

void
anv_magma_linux_physical_device_init_fd(struct anv_physical_device *device,
                                        const char *primary_path,
                                        const char *path,
                                        int fd)
{
   (void)fd;
   drmDevicePtr devices[8];
   int num_devices = 0;

   if (!primary_path && !path) {
      num_devices = drmGetDevices2(0, devices, ARRAY_SIZE(devices));
      if (num_devices > 0) {
         drmDevicePtr matched = NULL;
         for (int i = 0; i < num_devices; i++) {
            if (!(devices[i]->available_nodes & (1 << DRM_NODE_RENDER)))
               continue;
            if (devices[i]->bustype == DRM_BUS_PCI &&
                devices[i]->businfo.pci->domain == device->info.pci_domain &&
                devices[i]->businfo.pci->bus == device->info.pci_bus &&
                devices[i]->businfo.pci->dev == device->info.pci_dev &&
                devices[i]->businfo.pci->func == device->info.pci_func) {
               matched = devices[i];
               break;
            }
            if (!matched)
               matched = devices[i];
         }
         if (matched) {
            if (matched->available_nodes & (1 << DRM_NODE_PRIMARY))
               primary_path = matched->nodes[DRM_NODE_PRIMARY];
            if (matched->available_nodes & (1 << DRM_NODE_RENDER))
               path = matched->nodes[DRM_NODE_RENDER];
         }
      } else {
         primary_path = "/dev/dri/card0";
         path = "/dev/dri/renderD128";
      }
   }

   if (path && device->path[0] == '\0') {
      snprintf(device->path, ARRAY_SIZE(device->path), "%s", path);
   }

   int master_fd = -1;
   if (primary_path && device->instance->vk.enabled_extensions.KHR_display) {
      master_fd = open(primary_path, O_RDWR | O_CLOEXEC);
   }
   device->master_fd = master_fd;

   struct stat st;
   if (primary_path && stat(primary_path, &st) == 0) {
      device->has_master = true;
      device->master_major = major(st.st_rdev);
      device->master_minor = minor(st.st_rdev);
   } else {
      device->has_master = false;
      device->master_major = 0;
      device->master_minor = 0;
   }

   if (path && stat(path, &st) == 0) {
      device->has_local = true;
      device->local_major = major(st.st_rdev);
      device->local_minor = minor(st.st_rdev);
   } else {
      device->has_local = false;
      device->local_major = 0;
      device->local_minor = 0;
   }

   if (num_devices > 0)
      drmFreeDevices(devices, num_devices);
}

void
anv_magma_linux_physical_device_finish_fd(struct anv_physical_device *device)
{
   if (device->master_fd >= 0) {
      close(device->master_fd);
      device->master_fd = -1;
   }
}
