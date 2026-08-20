/*
 * Copyright © 2015 Intel Corporation
 *
 * Permission is hereby granted, free of charge, to any person obtaining a
 * copy of this software and associated documentation files (the "Software"),
 * to deal in the Software without restriction, including without limitation
 * the rights to use, copy, modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software, and to permit persons to whom the
 * Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice (including the next
 * paragraph) shall be included in all copies or substantial portions of the
 * Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL
 * THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
 * IN THE SOFTWARE.
 */

#include <errno.h>
#include <fcntl.h>
#include <sys/stat.h>
#ifdef MAJOR_IN_SYSMACROS
#include <sys/sysmacros.h>
#endif
#include <unistd.h>
#include <xf86drm.h>

#include "anv_private.h"
#include "common/intel_gem.h"
#include "common/xe/intel_gem.h"
#include "dev/intel_wa.h"
#include "dev/virtio/intel_virtio.h"
#include "util/os_mman.h"

VkResult
anv_linux_bo_unmap(struct anv_device *device, struct anv_bo *bo,
                   void *map, size_t map_size, bool replace)
{
   VG(VALGRIND_FREELIKE_BLOCK(map, 0));

   if (replace) {
      map = mmap(map, map_size, PROT_NONE,
                 MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED, -1, 0);
      if (map == MAP_FAILED) {
         return vk_errorf(device, VK_ERROR_MEMORY_MAP_FAILED,
                          "Failed to map over original mapping");
      }
   } else {
      munmap(map, map_size);
   }
   return VK_SUCCESS;
}

VkResult
anv_linux_physical_device_try_create(struct vk_instance *vk_instance,
                                     struct _drmDevice *drm_device,
                                     struct vk_physical_device **out)
{
   struct anv_instance *instance =
      container_of(vk_instance, struct anv_instance, vk);

   const char *primary_path = drm_device->nodes[DRM_NODE_PRIMARY];
   const char *path = drm_device->nodes[DRM_NODE_RENDER];
   VkResult result;
   int fd;
   int ret;

   fd = open(path, O_RDWR | O_CLOEXEC);
   if (fd < 0) {
      if (errno == ENOMEM) {
         return vk_errorf(instance, VK_ERROR_OUT_OF_HOST_MEMORY,
                          "Unable to open device %s: out of memory", path);
      }
      return vk_errorf(instance, VK_ERROR_INCOMPATIBLE_DRIVER,
                       "Unable to open device %s: %m", path);
   }

   ret = intel_virtio_init_fd(fd);
   if (ret < 0) {
      result = VK_ERROR_INCOMPATIBLE_DRIVER;
      goto fail_fd;
   }

   bool is_virtio = ret > 0;
   if (!is_virtio) {
      if (!(drm_device->available_nodes & (1 << DRM_NODE_RENDER)) ||
         drm_device->bustype != DRM_BUS_PCI ||
         drm_device->deviceinfo.pci->vendor_id != 0x8086) {
         result = VK_ERROR_INCOMPATIBLE_DRIVER;
         goto fail_fd;
      }
   }

   struct intel_device_info devinfo;
   if (!intel_get_device_info_from_fd(fd, &devinfo, 9, -1)) {
      result = VK_ERROR_INCOMPATIBLE_DRIVER;
      goto fail_fd;
   }

   if (devinfo.ver < 9) {
      result = VK_ERROR_INCOMPATIBLE_DRIVER;
      goto fail_fd;
   } else if (devinfo.probe_forced) {
   } else if (devinfo.ver > 30) {
      result = vk_errorf(instance, VK_ERROR_INCOMPATIBLE_DRIVER,
                         "Vulkan not yet supported on %s", devinfo.name);
      goto fail_fd;
   }

   if (devinfo.verx10 == 120)
      BITSET_CLEAR(devinfo.workarounds, INTEL_WA_16013994831);

   if (!devinfo.has_context_isolation) {
      result = vk_errorf(instance, VK_ERROR_INCOMPATIBLE_DRIVER,
                         "Vulkan requires context isolation for %s", devinfo.name);
      goto fail_fd;
   }

   result = anv_physical_device_create(instance, &devinfo, primary_path, path, fd, out);
   if (result != VK_SUCCESS)
      goto fail_fd;

   return VK_SUCCESS;

fail_fd:
   intel_virtio_unref_fd(fd);
   close(fd);
   return result;
}

void
anv_linux_physical_device_init_fd(struct anv_physical_device *device,
                                  const char *primary_path,
                                  const char *path,
                                  int fd)
{
   device->has_protected_contexts = device->info.ver >= 12 &&
      fd >= 0 && intel_gem_supports_protected_context(fd, device->info.kmd_type);

   device->has_huc = fd >= 0 && intel_gem_supports_huc(fd, device->info.kmd_type);

   uint64_t u64_ignore;
   device->has_reg_timestamp = fd >= 0 &&
      intel_gem_read_render_timestamp(fd, device->info.kmd_type, &u64_ignore);

   device->can_get_vm_faults =
      !device->has_scratch_page && fd >= 0 &&
      xe_gem_supports_get_vm_faults(device->local_fd);

   int master_fd = -1;
   if (primary_path && device->instance->vk.enabled_extensions.KHR_display) {
      master_fd = open(primary_path, O_RDWR | O_CLOEXEC);
      if (master_fd >= 0) {
         if (!intel_gem_can_render_on_fd(master_fd, device->info.kmd_type)) {
            close(master_fd);
            master_fd = -1;
         }
      }
   }
   device->master_fd = master_fd;

   if (fd >= 0) {
      device->engine_info = intel_engine_get_info(fd, device->info.kmd_type);
      intel_common_update_device_info(fd, &device->info);
   }

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
}

void
anv_linux_physical_device_finish_fd(struct anv_physical_device *device)
{
   if (device->local_fd >= 0) {
      intel_virtio_unref_fd(device->local_fd);
      close(device->local_fd);
   }
   if (device->master_fd >= 0)
      close(device->master_fd);
}

VkResult
anv_linux_device_open(struct anv_device *device,
                      const struct anv_physical_device *physical_device)
{
   if (physical_device->path[0] == '\0') {
      device->fd = -1;
      return VK_SUCCESS;
   }

   device->fd = open(physical_device->path, O_RDWR | O_CLOEXEC);
   if (device->fd == -1)
      return vk_error(device, VK_ERROR_INITIALIZATION_FAILED);

   if (intel_virtio_init_fd(device->fd) < 0) {
      close(device->fd);
      device->fd = -1;
      return VK_ERROR_INCOMPATIBLE_DRIVER;
   }

   return VK_SUCCESS;
}

void
anv_linux_device_close(struct anv_device *device)
{
   if (device->fd >= 0) {
      intel_virtio_unref_fd(device->fd);
      close(device->fd);
      device->fd = -1;
   }
}

VkResult
anv_linux_device_map_placed_addr_slab(struct anv_device *device,
                                      struct anv_bo *bo,
                                      uint64_t offset,
                                      size_t size,
                                      void *placed_addr,
                                      void **map_out)
{
   int prime_handle =
      device->kmd_backend->linux_sys_driver_ops->bo_export_fd(device, bo);
   VkResult result = VK_SUCCESS;

   if (prime_handle < 0) {
      return vk_errorf(device, VK_ERROR_MEMORY_MAP_FAILED,
                       "bo_export_fd() before mmap failed: %m");
   }

   offset += (bo->offset - bo->slab_parent->offset);
   void *map = os_mmap(placed_addr,
                       size,
                       PROT_READ | PROT_WRITE,
                       MAP_FIXED | MAP_SHARED,
                       prime_handle,
                       offset);
   if (map == MAP_FAILED) {
      result = vk_errorf(device, VK_ERROR_MEMORY_MAP_FAILED, "mmap failed: %m");
      goto end;
   }

   assert(placed_addr == NULL || map == placed_addr);
   assert(map != NULL);
   VG(VALGRIND_MALLOCLIKE_BLOCK(map, size, 0, 1));

   if (map_out)
      *map_out = map;

end:
   close(prime_handle);
   return result;
}

VkResult
anv_linux_device_import_bo(struct anv_device *device,
                           int fd,
                           enum anv_bo_alloc_flags alloc_flags,
                           uint64_t client_address,
                           struct anv_bo **bo_out)
{
   assert(!(alloc_flags & (ANV_BO_ALLOC_MAPPED |
                           ANV_BO_ALLOC_HOST_CACHED |
                           ANV_BO_ALLOC_HOST_COHERENT |
                           ANV_BO_ALLOC_FIXED_ADDRESS)));
   assert(alloc_flags & ANV_BO_ALLOC_EXTERNAL);

   struct anv_bo_cache *cache = &device->bo_cache;

   simple_mtx_lock(&cache->mutex);

   uint32_t gem_handle =
      device->kmd_backend->linux_sys_driver_ops->bo_import_fd(device, fd);
   if (!gem_handle) {
      simple_mtx_unlock(&cache->mutex);
      return vk_error(device, VK_ERROR_INVALID_EXTERNAL_HANDLE);
   }

   struct anv_bo *bo = anv_device_lookup_bo(device, gem_handle);

   uint32_t bo_flags;
   VkResult result =
      device->kmd_backend->import_bo_alloc_flags_to_bo_flags(device, bo,
                                                             alloc_flags,
                                                             &bo_flags);
   if (result != VK_SUCCESS) {
      simple_mtx_unlock(&cache->mutex);
      return result;
   }

   if (bo->refcount > 0) {
      if (client_address && client_address != intel_48b_address(bo->offset)) {
         simple_mtx_unlock(&cache->mutex);
         return vk_errorf(device, VK_ERROR_INVALID_EXTERNAL_HANDLE,
                          "The same BO was imported at two different "
                          "addresses");
      }

      __sync_fetch_and_add(&bo->refcount, 1);
   } else {
      alloc_flags |= ANV_BO_ALLOC_IMPORTED;
      struct anv_bo new_bo = {
         .name = "imported",
         .gem_handle = gem_handle,
         .refcount = 1,
         .offset = -1,
         .alloc_flags = alloc_flags,
      };

      off_t size = lseek(fd, 0, SEEK_END);
      if (size == (off_t)-1) {
         device->kmd_backend->gem_close(device, &new_bo);
         simple_mtx_unlock(&cache->mutex);
         return vk_error(device, VK_ERROR_INVALID_EXTERNAL_HANDLE);
      }
      new_bo.size = size;
      new_bo.actual_size = size;

      uint32_t alignment = anv_bo_vma_calc_alignment_requirement(device, alloc_flags, size);
      VkResult result = anv_bo_vma_alloc_or_close(device, &new_bo,
                                                  alloc_flags,
                                                  client_address,
                                                  alignment);
      if (result != VK_SUCCESS) {
         simple_mtx_unlock(&cache->mutex);
         return result;
      }

      result = device->kmd_backend->vm_bind_bo(device, &new_bo);
      if (result != VK_SUCCESS) {
         anv_bo_vma_free(device, &new_bo);
         simple_mtx_unlock(&cache->mutex);
         return result;
      }

      *bo = new_bo;

      ANV_RMV(bo_allocate, device, bo);
   }

   bo->flags = bo_flags;

   simple_mtx_unlock(&cache->mutex);
   *bo_out = bo;

   return VK_SUCCESS;
}

VkResult
anv_linux_device_export_bo(struct anv_device *device,
                           struct anv_bo *bo, int *fd_out)
{
   assert(anv_device_lookup_bo(device, bo->gem_handle) == bo);

   /* This BO must have been flagged external in order for us to be able
    * to export it.  This is done based on external options passed into
    * anv_AllocateMemory.
    */
   assert(anv_bo_is_external(bo));

   int fd = device->kmd_backend->linux_sys_driver_ops->bo_export_fd(device, bo);
   if (fd < 0)
      return vk_error(device, VK_ERROR_TOO_MANY_OBJECTS);

   *fd_out = fd;

   return VK_SUCCESS;
}

VkResult anv_GetMemoryFdKHR(
    VkDevice                                    device_h,
    const VkMemoryGetFdInfoKHR*                 pGetFdInfo,
    int*                                        pFd)
{
   ANV_FROM_HANDLE(anv_device, dev, device_h);
   ANV_FROM_HANDLE(anv_device_memory, mem, pGetFdInfo->memory);

   assert(pGetFdInfo->sType == VK_STRUCTURE_TYPE_MEMORY_GET_FD_INFO_KHR);

   assert(pGetFdInfo->handleType == VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT ||
          pGetFdInfo->handleType == VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT);

   return anv_linux_device_export_bo(dev, mem->bo, pFd);
}

VkResult anv_GetMemoryFdPropertiesKHR(
    VkDevice                                    _device,
    VkExternalMemoryHandleTypeFlagBits          handleType,
    int                                         fd,
    VkMemoryFdPropertiesKHR*                    pMemoryFdProperties)
{
   ANV_FROM_HANDLE(anv_device, device, _device);

   switch (handleType) {
   case VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT:
      /* dma-buf can be imported as any memory type */
      pMemoryFdProperties->memoryTypeBits =
         (1 << device->physical->memory.type_count) - 1;
      return VK_SUCCESS;

   default:
      /* The valid usage section for this function says:
       *
       *    "handleType must not be one of the handle types defined as
       *    opaque."
       *
       * So opaque handle types fall into the default "unsupported" case.
       */
      return vk_error(device, VK_ERROR_INVALID_EXTERNAL_HANDLE);
   }
}
