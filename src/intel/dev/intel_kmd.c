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

#include <string.h>
#include "util/libdrm.h"
#include "util/log.h"
#include "util/u_debug.h"

#include "common/intel_gem.h"
#include "i915/intel_device_info.h"
#include "xe/intel_device_info.h"
#include "virtio/intel_virtio.h"

#include "intel_device_info.h"
#include "intel_hwconfig.h"
#include "intel_kmd.h"
#include "intel_wa.h"

enum intel_kmd_type
intel_get_kmd_type(int fd)
{
   enum intel_kmd_type type = INTEL_KMD_TYPE_INVALID;
   drmVersionPtr version = drmGetVersion(fd);

   if (!version)
      return type;

   /*
    * For virtio, version->name would be either "virtio_gpu" or "i915",
    * depending on whether vtest is used or not.
    */
   if (strcmp(version->name, "i915") == 0 || is_intel_virtio_fd(fd))
      type = INTEL_KMD_TYPE_I915;
   else if (strcmp(version->name, "xe") == 0)
      type = INTEL_KMD_TYPE_XE;

   drmFreeVersion(version);

   return type;
}

bool
intel_get_device_info_from_fd(int fd, struct intel_device_info *devinfo, int min_ver, int max_ver)
{
   if (NULL != os_get_option("INTEL_STUB_GPU_JSON")) {
      /* This call will succeed when shim-drm has been initialized with a
       * serialized intel_device_info structure.
       */
      struct drm_intel_stub_devinfo arg = {
         .addr = (uintptr_t)devinfo,
         .size = sizeof(*devinfo),
      };
      if (0 == intel_ioctl(fd, DRM_IOCTL_INTEL_STUB_DEVINFO, &arg)) {
         intel_device_info_init_was(devinfo);
         intel_device_info_apply_workarounds(devinfo);
         return true;
      }
   }

   /* Get PCI info.
    *
    * Some callers may already have a valid drm device which holds values of
    * PCI fields queried here prior to calling this function. But making this
    * query optional leads to a more cumbersome implementation. These callers
    * still need to initialize the fields somewhere out of this function and
    * rely on an ioctl to get PCI device id for the next step when skipping
    * this drm query.
    */
   if (is_intel_virtio_fd(fd)) {
      if (!intel_virtio_get_pci_device_info(fd, devinfo))
         return false;

      if (!intel_device_info_init_common(devinfo->pci_device_id,
                                         false, devinfo))
         return false;

      devinfo->is_virtio = true;
   } else {
      drmDevicePtr drmdev = NULL;
      if (drmGetDevice2(fd, DRM_DEVICE_GET_PCI_REVISION, &drmdev)) {
         mesa_loge("Failed to query drm device.");
         return false;
      }
      if (!intel_device_info_init_common(drmdev->deviceinfo.pci->device_id,
                                         false, devinfo)) {
         drmFreeDevice(&drmdev);
         return false;
      }

      devinfo->pci_domain = drmdev->businfo.pci->domain;
      devinfo->pci_bus = drmdev->businfo.pci->bus;
      devinfo->pci_dev = drmdev->businfo.pci->dev;
      devinfo->pci_func = drmdev->businfo.pci->func;
      devinfo->pci_device_id = drmdev->deviceinfo.pci->device_id;
      devinfo->pci_revision_id = drmdev->deviceinfo.pci->revision_id;
      drmFreeDevice(&drmdev);
   }

   if ((min_ver > 0 && devinfo->ver < min_ver) || (max_ver > 0 && devinfo->ver > max_ver))
      return false;

   devinfo->no_hw = debug_get_bool_option("INTEL_NO_HW", false);

   devinfo->kmd_type = intel_get_kmd_type(fd);
   if (devinfo->kmd_type == INTEL_KMD_TYPE_INVALID) {
      mesa_loge("Unknown kernel mode driver");
      return false;
   }

   /* remaining initialization queries the kernel for device info */
   if (devinfo->no_hw) {
      /* Provide some sensible values for NO_HW. */
      devinfo->gtt_size =
         devinfo->ver >= 8 ? (1ull << 48) : 2ull * 1024 * 1024 * 1024;
      intel_device_info_compute_system_memory(devinfo, false);
      return true;
   }

   bool ret;
   switch (devinfo->kmd_type) {
   case INTEL_KMD_TYPE_I915:
      ret = intel_device_info_i915_get_info_from_fd(fd, devinfo);
      break;
   case INTEL_KMD_TYPE_XE:
      ret = intel_device_info_xe_get_info_from_fd(fd, devinfo);
      if (devinfo->verx10 < 200) {
         if (!debug_get_bool_option("INTEL_XE_IGNORE_EXPERIMENTAL_WARNING", false))
            mesa_logw("Support for this platform is experimental with Xe KMD, bug reports may be ignored.");
      }
      break;
   default:
      ret = false;
      UNREACHABLE("Missing");
   }
   if (!ret) {
      mesa_logw("Could not get intel_device_info.");
      return false;
   }

   /* region info is required for lmem support */
   if (devinfo->has_local_mem && !devinfo->mem.use_class_instance) {
      mesa_logw("Could not query local memory size.");
      return false;
   }

   intel_device_info_finish_init(devinfo);

   return true;
}

bool
intel_device_info_update_memory_info(struct intel_device_info *devinfo, int fd)
{
   bool ret;

   switch (devinfo->kmd_type) {
   case INTEL_KMD_TYPE_I915:
      ret = intel_device_info_i915_query_regions(devinfo, fd, true);
      break;
   case INTEL_KMD_TYPE_XE:
      ret = intel_device_info_xe_query_regions(fd, devinfo, true);
      break;
   default:
      ret = false;
   }

   if (ret)
      intel_device_info_adjust_memory(devinfo);
   return ret;
}

static struct hwconfig *
intel_get_hwconfig_table(int fd, struct intel_device_info *devinfo,
                         int32_t *hwconfig_len)
{
   switch (devinfo->kmd_type) {
   case INTEL_KMD_TYPE_I915:
      return intel_device_info_i915_query_hwconfig(fd, hwconfig_len);
   case INTEL_KMD_TYPE_XE:
      return intel_device_info_xe_query_hwconfig(fd, hwconfig_len);
   default:
      UNREACHABLE("unknown kmd type");
      return NULL;
   }
}

void
intel_get_and_print_hwconfig_table(int fd, struct intel_device_info *devinfo)
{
   struct hwconfig *hwconfig;
   int32_t hwconfig_len = 0;

   hwconfig = intel_get_hwconfig_table(fd, devinfo, &hwconfig_len);
   if (hwconfig) {
      intel_print_hwconfig_table(hwconfig, hwconfig_len);
      free(hwconfig);
   }
}

void
intel_check_hwconfig_items(int fd, struct intel_device_info *devinfo)
{
   struct hwconfig *data;
   int32_t len = 0;

   data = intel_get_hwconfig_table(fd, devinfo, &len);
   if (data) {
      intel_check_hwconfig_table(devinfo, data, len);
      free(data);
   }
}
