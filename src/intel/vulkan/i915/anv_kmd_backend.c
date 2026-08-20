/*
 * Copyright © 2023 Intel Corporation
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

#include <sys/mman.h>

#include "anv_private.h"

#include "i915/anv_batch_chain.h"
#include "i915/anv_device.h"
#include "i915/anv_queue.h"

#include "drm-uapi/i915_drm.h"
#include "intel/common/i915/intel_gem.h"

static int
i915_gem_set_caching(struct anv_device *device,
                     uint32_t gem_handle, uint32_t caching)
{
   struct drm_i915_gem_caching gem_caching = {
      .handle = gem_handle,
      .caching = caching,
   };

   return intel_ioctl(device->fd, DRM_IOCTL_I915_GEM_SET_CACHING, &gem_caching);
}

static uint32_t
i915_gem_create(struct anv_device *device,
                const struct intel_memory_class_instance **regions,
                uint16_t num_regions, uint64_t size,
                enum anv_bo_alloc_flags alloc_flags,
                uint64_t *actual_size)
{
   if (unlikely(!device->info->mem.use_class_instance)) {
      assert(num_regions == 1 &&
             device->physical->sys.region == regions[0]);

      struct drm_i915_gem_create gem_create = {
            .size = size,
      };
      if (intel_ioctl(device->fd, DRM_IOCTL_I915_GEM_CREATE, &gem_create))
         return 0;

      if ((alloc_flags & ANV_BO_ALLOC_HOST_CACHED_COHERENT) == ANV_BO_ALLOC_HOST_CACHED_COHERENT) {
         /* We don't want to change these defaults if it's going to be shared
          * with another process.
          */
         assert(!(alloc_flags & ANV_BO_ALLOC_EXTERNAL));

         /* Regular objects are created I915_CACHING_CACHED on LLC platforms and
          * I915_CACHING_NONE on non-LLC platforms.  For many internal state
          * objects, we'd rather take the snooping overhead than risk forgetting
          * a CLFLUSH somewhere.  Userptr objects are always created as
          * I915_CACHING_CACHED, which on non-LLC means snooped so there's no
          * need to do this there.
          */
         if (device->info->has_caching_uapi && !device->info->has_llc)
            i915_gem_set_caching(device, gem_create.handle, I915_CACHING_CACHED);
      }

      *actual_size = gem_create.size;
      return gem_create.handle;
   }

   struct drm_i915_gem_memory_class_instance i915_regions[2];
   assert(num_regions <= ARRAY_SIZE(i915_regions));

   for (uint16_t i = 0; i < num_regions; i++) {
      i915_regions[i].memory_class = regions[i]->klass;
      i915_regions[i].memory_instance = regions[i]->instance;
   }

   uint32_t flags = 0;
   if (alloc_flags & (ANV_BO_ALLOC_MAPPED | ANV_BO_ALLOC_LOCAL_MEM_CPU_VISIBLE) &&
       !(alloc_flags & ANV_BO_ALLOC_NO_LOCAL_MEM))
      if (device->physical->vram_non_mappable.size > 0)
         flags |= I915_GEM_CREATE_EXT_FLAG_NEEDS_CPU_ACCESS;

   struct drm_i915_gem_create_ext_memory_regions ext_regions = {
      .num_regions = num_regions,
      .regions = (uintptr_t)i915_regions,
   };
   struct drm_i915_gem_create_ext gem_create = {
      .size = size,
      .flags = flags,
   };

   intel_i915_gem_add_ext(&gem_create.extensions,
                          I915_GEM_CREATE_EXT_MEMORY_REGIONS,
                          &ext_regions.base);

   struct drm_i915_gem_create_ext_set_pat set_pat_param = { 0 };
   if (device->info->has_set_pat_uapi) {
      /* Set PAT param */
      set_pat_param.pat_index = anv_device_get_pat_entry(device, alloc_flags)->index;
      intel_i915_gem_add_ext(&gem_create.extensions,
                             I915_GEM_CREATE_EXT_SET_PAT,
                             &set_pat_param.base);
   }

   struct drm_i915_gem_create_ext_protected_content protected_param = { 0 };
   if (alloc_flags & ANV_BO_ALLOC_PROTECTED) {
      intel_i915_gem_add_ext(&gem_create.extensions,
                             I915_GEM_CREATE_EXT_PROTECTED_CONTENT,
                             &protected_param.base);
   }

   if (intel_ioctl(device->fd, DRM_IOCTL_I915_GEM_CREATE_EXT, &gem_create))
      return 0;

   *actual_size = gem_create.size;

   if ((alloc_flags & ANV_BO_ALLOC_HOST_CACHED_COHERENT) == ANV_BO_ALLOC_HOST_CACHED_COHERENT) {
      /* We don't want to change these defaults if it's going to be shared
       * with another process.
       */
      assert(!(alloc_flags & ANV_BO_ALLOC_EXTERNAL));

      /* Regular objects are created I915_CACHING_CACHED on LLC platforms and
       * I915_CACHING_NONE on non-LLC platforms.  For many internal state
       * objects, we'd rather take the snooping overhead than risk forgetting
       * a CLFLUSH somewhere.  Userptr objects are always created as
       * I915_CACHING_CACHED, which on non-LLC means snooped so there's no
       * need to do this there.
       */
      if (device->info->has_caching_uapi && !device->info->has_llc)
         i915_gem_set_caching(device, gem_create.handle, I915_CACHING_CACHED);
   }

   return gem_create.handle;
}

static void
i915_gem_close(struct anv_device *device, struct anv_bo *bo)
{
   struct drm_gem_close close = {
      .handle = bo->gem_handle,
   };

   intel_ioctl(device->fd, DRM_IOCTL_GEM_CLOSE, &close);
}

static void *
i915_gem_mmap_offset(struct anv_device *device, struct anv_bo *bo,
                     uint64_t offset, uint64_t size, uint32_t flags,
                     void *placed_addr)
{
   struct drm_i915_gem_mmap_offset gem_mmap = {
      .handle = bo->gem_handle,
      .flags = flags,
   };
   if (intel_ioctl(device->fd, DRM_IOCTL_I915_GEM_MMAP_OFFSET, &gem_mmap))
      return MAP_FAILED;

   if (placed_addr != NULL) {
      const uint64_t placed_num = (uintptr_t)placed_addr;

      assert(placed_num >= offset);
      if (placed_num < offset)
         return NULL;

      placed_addr -= offset;
   }

   /* The Kernel uAPI doesn't allow us to map with an offset. To work around,
    * overallocate and then unmap the unneeded region
    */
   void *ptr;

   if (device->physical->info.is_virtio)
      ptr = intel_virtio_bo_mmap(device->fd, bo->gem_handle,
                                 offset + size, placed_addr);
   else
      ptr = mmap(placed_addr, offset + size,
                 PROT_READ | PROT_WRITE,
                 (placed_addr != NULL ? MAP_FIXED : 0) | MAP_SHARED,
                 device->fd, gem_mmap.offset);

   if (ptr == MAP_FAILED)
      return ptr;

   void *ret = ptr + offset;

   if (offset != 0)
      munmap(ptr, offset);

   return ret;
}

static void *
i915_gem_mmap_legacy(struct anv_device *device, struct anv_bo *bo, uint64_t offset,
                      uint64_t size, uint32_t flags)
{
   struct drm_i915_gem_mmap gem_mmap = {
      .handle = bo->gem_handle,
      .offset = offset,
      .size = size,
      .flags = flags,
   };
   if (intel_ioctl(device->fd, DRM_IOCTL_I915_GEM_MMAP, &gem_mmap))
      return MAP_FAILED;

   return (void *)(uintptr_t) gem_mmap.addr_ptr;
}

static enum intel_device_info_mmap_mode
anv_bo_get_mmap_mode(struct anv_device *device, struct anv_bo *bo)
{
   enum anv_bo_alloc_flags alloc_flags = bo->alloc_flags;

   if (device->info->has_set_pat_uapi)
      return anv_device_get_pat_entry(device, alloc_flags)->mmap;

   if (anv_physical_device_has_vram(device->physical)) {
      if ((alloc_flags & ANV_BO_ALLOC_NO_LOCAL_MEM) ||
          (alloc_flags & ANV_BO_ALLOC_IMPORTED))
         return INTEL_DEVICE_INFO_MMAP_MODE_WB;

      return INTEL_DEVICE_INFO_MMAP_MODE_WC;
   }

   /* gfx9 atom */
   if (!device->info->has_llc) {
      /* user wants a cached and coherent memory but to achieve it without
       * LLC in older platforms DRM_IOCTL_I915_GEM_SET_CACHING needs to be
       * supported and set.
       */
      if (alloc_flags & ANV_BO_ALLOC_HOST_CACHED)
         return INTEL_DEVICE_INFO_MMAP_MODE_WB;

      return INTEL_DEVICE_INFO_MMAP_MODE_WC;
   }

   if (alloc_flags & (ANV_BO_ALLOC_SCANOUT | ANV_BO_ALLOC_EXTERNAL))
      return INTEL_DEVICE_INFO_MMAP_MODE_WC;

   return INTEL_DEVICE_INFO_MMAP_MODE_WB;
}

static uint32_t
mmap_calc_flags(struct anv_device *device, struct anv_bo *bo)
{
   if (device->info->has_local_mem)
      return I915_MMAP_OFFSET_FIXED;

   uint32_t flags;
   switch (anv_bo_get_mmap_mode(device, bo)) {
   case INTEL_DEVICE_INFO_MMAP_MODE_WC:
      flags = I915_MMAP_WC;
      break;
   default:
      /* no flags == WB */
      flags = 0;
   }

   if (likely(device->physical->info.has_mmap_offset))
      flags = (flags & I915_MMAP_WC) ? I915_MMAP_OFFSET_WC : I915_MMAP_OFFSET_WB;
   return flags;
}

static void *
i915_gem_mmap(struct anv_device *device, struct anv_bo *bo, uint64_t offset,
              uint64_t size, void *placed_addr)
{
   const uint32_t flags = mmap_calc_flags(device, bo);

   if (likely(device->physical->info.has_mmap_offset))
      return i915_gem_mmap_offset(device, bo, offset, size, flags, placed_addr);
   assert(placed_addr == NULL);
   return i915_gem_mmap_legacy(device, bo, offset, size, flags);
}

static VkResult
i915_vm_bind(struct anv_device *device, struct anv_sparse_submission *submit,
             enum anv_vm_bind_flags flags)
{
   return VK_SUCCESS;
}

static VkResult
i915_vm_bind_bo(struct anv_device *device, struct anv_bo *bo)
{
   return VK_SUCCESS;
}

static uint32_t
i915_gem_create_userptr(struct anv_device *device, void *mem, uint64_t size)
{
   struct drm_i915_gem_userptr userptr = {
      .user_ptr = (__u64)((unsigned long) mem),
      .user_size = size,
      .flags = 0,
   };

   if (device->physical->info.has_userptr_probe)
      userptr.flags |= I915_USERPTR_PROBE;

   int ret = intel_ioctl(device->fd, DRM_IOCTL_I915_GEM_USERPTR, &userptr);
   if (ret == -1)
      return 0;

   return userptr.handle;
}

static uint32_t
i915_bo_alloc_flags_to_bo_flags(struct anv_device *device,
                                enum anv_bo_alloc_flags alloc_flags)
{
   uint64_t bo_flags = EXEC_OBJECT_PINNED;

   if (!(alloc_flags & ANV_BO_ALLOC_32BIT_ADDRESS))
      bo_flags |= EXEC_OBJECT_SUPPORTS_48B_ADDRESS;

   if ((alloc_flags & ANV_BO_ALLOC_CAPTURE) ||
        INTEL_DEBUG(DEBUG_CAPTURE_ALL))
      bo_flags |= EXEC_OBJECT_CAPTURE;

   if (alloc_flags & ANV_BO_ALLOC_IMPLICIT_WRITE) {
      assert(alloc_flags & ANV_BO_ALLOC_IMPLICIT_SYNC);
      bo_flags |= EXEC_OBJECT_WRITE;
   }

   if (!(alloc_flags & ANV_BO_ALLOC_IMPLICIT_SYNC))
      bo_flags |= EXEC_OBJECT_ASYNC;

   return bo_flags;
}

static VkResult
i915_get_bo_tiling(struct anv_device *device, struct anv_bo *bo,
                   enum isl_tiling *tiling_out)
{
   if (!device->info->has_tiling_uapi) {
      return vk_errorf(device, VK_ERROR_INVALID_EXTERNAL_HANDLE,
                       "failed to get BO tiling: %m");
   }

   struct drm_i915_gem_get_tiling get_tiling = {
      .handle = bo->gem_handle,
   };

   /* FIXME: On discrete platforms we don't have DRM_IOCTL_I915_GEM_GET_TILING
    * anymore, so we will need another way to get the tiling. Apparently this
    * is only used in Android code, so we may need some other way to
    * communicate the tiling mode.
    */
   if (intel_ioctl(device->fd, DRM_IOCTL_I915_GEM_GET_TILING, &get_tiling)) {
      assert(!"Failed to get BO tiling");
      return vk_errorf(device, VK_ERROR_INVALID_EXTERNAL_HANDLE,
                       "failed to get BO tiling: %m");
   }

   *tiling_out = isl_tiling_from_i915_tiling(get_tiling.tiling_mode);

   return VK_SUCCESS;
}

static VkResult
i915_set_bo_tiling(struct anv_device *device, struct anv_bo *bo,
                   uint32_t row_pitch_B, enum isl_tiling tiling)
{
   assert(bo->slab_parent == NULL);

   /* On discrete platforms we don't have DRM_IOCTL_I915_GEM_SET_TILING. So
    * nothing needs to be done.
    */
   if (!device->info->has_tiling_uapi)
      return VK_SUCCESS;

   /* set_tiling overwrites the input on the error path, so we have to open
    * code intel_ioctl.
    */
   struct drm_i915_gem_set_tiling set_tiling = {
      .handle = bo->gem_handle,
      .tiling_mode = isl_tiling_to_i915_tiling(tiling),
      .stride = row_pitch_B,
   };

   if (intel_ioctl(device->fd, DRM_IOCTL_I915_GEM_SET_TILING, &set_tiling)) {
      return vk_errorf(device, VK_ERROR_OUT_OF_DEVICE_MEMORY,
                       "failed to set BO tiling: %m");
   }

   return VK_SUCCESS;
}

static VkResult
i915_import_bo_alloc_flags_to_bo_flags(struct anv_device *device,
                                       struct anv_bo *bo,
                                       enum anv_bo_alloc_flags alloc_flags,
                                       uint32_t *out_bo_flags)
{
   const uint32_t bo_flags =
         i915_bo_alloc_flags_to_bo_flags(device, alloc_flags);
   if (bo->refcount == 0) {
      *out_bo_flags = bo_flags;
      return VK_SUCCESS;
   }

   /* We have to be careful how we combine flags so that it makes sense.
    * Really, though, if we get to this case and it actually matters, the
    * client has imported a BO twice in different ways and they get what
    * they have coming.
    */
   uint32_t new_flags = 0;
   new_flags |= (bo->flags | bo_flags) & EXEC_OBJECT_WRITE;
   new_flags |= (bo->flags & bo_flags) & EXEC_OBJECT_ASYNC;
   new_flags |= (bo->flags & bo_flags) & EXEC_OBJECT_SUPPORTS_48B_ADDRESS;
   new_flags |= (bo->flags | bo_flags) & EXEC_OBJECT_PINNED;
   new_flags |= (bo->flags | bo_flags) & EXEC_OBJECT_CAPTURE;

   /* It's theoretically possible for a BO to get imported such that it's
    * both pinned and not pinned.  The only way this can happen is if it
    * gets imported as both a semaphore and a memory object and that would
    * be an application error.  Just fail out in that case.
    */
   if ((bo->flags & EXEC_OBJECT_PINNED) !=
       (bo_flags & EXEC_OBJECT_PINNED))
      return vk_errorf(device, VK_ERROR_INVALID_EXTERNAL_HANDLE,
                       "The same BO was imported two different ways");

   /* It's also theoretically possible that someone could export a BO from
    * one heap and import it into another or to import the same BO into two
    * different heaps.  If this happens, we could potentially end up both
    * allowing and disallowing 48-bit addresses.  There's not much we can
    * do about it if we're pinning so we just throw an error and hope no
    * app is actually that stupid.
    */
   if ((new_flags & EXEC_OBJECT_PINNED) &&
       (bo->flags & EXEC_OBJECT_SUPPORTS_48B_ADDRESS) !=
       (bo_flags & EXEC_OBJECT_SUPPORTS_48B_ADDRESS))
      return vk_errorf(device, VK_ERROR_INVALID_EXTERNAL_HANDLE,
                       "The same BO was imported on two different heaps");

   *out_bo_flags = new_flags;
   return VK_SUCCESS;
}

static uint32_t
i915_queue_get_exec_id(struct anv_queue *queue)
{
   struct anv_device *device = queue->device;
   return device->physical->has_vm_control ?
          queue->context_id : device->context_id;
}

static const struct anv_linux_sys_driver_ops i915_linux_sys_driver_ops = {
   .bo_export_fd = anv_gem_bo_export_fd,
   .bo_import_fd = anv_gem_bo_import_fd,
   .get_bo_tiling = i915_get_bo_tiling,
   .set_bo_tiling = i915_set_bo_tiling,
};

const struct anv_kmd_backend *
anv_i915_kmd_backend_get(void)
{
   static const struct anv_kmd_backend i915_backend = {
      .linux_sys_driver_ops = &i915_linux_sys_driver_ops,
      .gem_create = i915_gem_create,
      .gem_create_userptr = i915_gem_create_userptr,
      .gem_close = i915_gem_close,
      .gem_mmap = i915_gem_mmap,
      .bo_unmap = anv_linux_bo_unmap,
      .vm_bind = i915_vm_bind,
      .vm_bind_bo = i915_vm_bind_bo,
      .vm_unbind_bo = i915_vm_bind_bo,
      .queue_exec_locked = i915_queue_exec_locked,
      .queue_exec_async = i915_queue_exec_async,
      .queue_get_exec_id = i915_queue_get_exec_id,
      .bo_alloc_flags_to_bo_flags = i915_bo_alloc_flags_to_bo_flags,
      .import_bo_alloc_flags_to_bo_flags = i915_import_bo_alloc_flags_to_bo_flags,
      .physical_device_get_parameters = anv_i915_physical_device_get_parameters,
      .physical_device_init_memory_types = anv_i915_physical_device_init_memory_types,
      .physical_device_init_sync = anv_i915_physical_device_init_sync,
      .device_open = anv_linux_device_open,
      .device_close = anv_linux_device_close,
      .device_setup_context_or_vm = anv_i915_device_setup_context,
      .device_destroy_context_or_vm = anv_i915_device_destroy_context,
      .device_check_status = anv_i915_device_check_status,
      .create_engine = anv_i915_create_engine,
      .destroy_engine = anv_i915_destroy_engine,
   };
   return &i915_backend;
}
