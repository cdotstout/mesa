/*
 * Copyright 2026 The Magma GPU Project
 * SPDX-License-Identifier: MIT
 */

#include <stdio.h>
#include <string.h>

#include <magma.h>

#include "dev/magma/intel_device_info.h"
#include "dev/intel_device_info.h"
#include "dev/intel_hwconfig.h"
#include "dev/intel_hwconfig_types.h"
#include "dev/intel_wa.h"

#include "util/log.h"
#include "util/u_debug.h"

static void
magma_set_device_name(struct intel_device_info *devinfo)
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

bool
intel_device_info_magma_get_info(magma_physical_device_t pdev,
                                 struct intel_device_info *devinfo,
                                 int min_ver, int max_ver)
{
   struct magma_intel_device_info intel_info = {
      .header = {
         .s_type = MAGMA_STRUCTURE_TYPE_INTEL_DEVICE_INFO,
         .size = sizeof(intel_info),
         .p_next = NULL,
      },
   };
   struct magma_timestamp_properties ts_props = {
      .header = {
         .s_type = MAGMA_STRUCTURE_TYPE_TIMESTAMP_PROPERTIES,
         .size = sizeof(ts_props),
         .p_next = &intel_info,
      },
   };
   struct magma_address_space_properties as_props = {
      .header = {
         .s_type = MAGMA_STRUCTURE_TYPE_ADDRESS_SPACE_PROPERTIES,
         .size = sizeof(as_props),
         .p_next = &ts_props,
      },
   };
   struct magma_memory_properties props = {
      .header = {
         .s_type = MAGMA_STRUCTURE_TYPE_MEMORY_PROPERTIES,
         .size = sizeof(props),
         .p_next = &as_props,
      },
   };
   struct magma_physical_device_info info = {
      .header = {
         .s_type = MAGMA_STRUCTURE_TYPE_PHYSICAL_DEVICE_INFO,
         .size = sizeof(info),
         .p_next = &props,
      },
   };

   if (magma_physical_device_query(pdev, &info) != MAGMA_STATUS_SUCCESS)
      return false;

   if (info.vendor_id != MAGMA_VENDOR_ID_INTEL &&
       info.vendor_id != (magma_vendor_id_t)0x8086)
      return false;

   if (!intel_device_info_init_common(info.device_id, false, devinfo))
      return false;

   devinfo->pci_domain = info.pci_bus_info.domain;
   devinfo->pci_bus = info.pci_bus_info.bus;
   devinfo->pci_dev = info.pci_bus_info.device;
   devinfo->pci_func = info.pci_bus_info.function;
   devinfo->pci_device_id = info.device_id;
   devinfo->pci_revision_id = info.pci_bus_info.revision_id;

   if (intel_info.ip_ver_major > 0) {
      devinfo->gfx_ip_ver = GFX_IP_VER(intel_info.ip_ver_major,
                                       intel_info.ip_ver_minor);
      devinfo->revision = intel_info.ip_ver_rev;
   }

   if ((min_ver > 0 && devinfo->ver < min_ver) ||
       (max_ver > 0 && devinfo->ver > max_ver))
      return false;

   devinfo->no_hw = debug_get_bool_option("INTEL_NO_HW", false);
   devinfo->kmd_type = INTEL_KMD_TYPE_MAGMA;
   magma_set_device_name(devinfo);

   if (intel_info.hw_config_count > 0) {
      intel_hwconfig_process_table(devinfo, intel_info.hw_config_data,
                                   intel_info.hw_config_count * sizeof(uint32_t));
      for (uint32_t i = 0; i + 2 < intel_info.hw_config_count; ) {
         uint32_t key = intel_info.hw_config_data[i];
         uint32_t len = intel_info.hw_config_data[i + 1];
         if ((key == INTEL_HWCONFIG_MAX_EU_PER_SUBSLICE ||
              key == INTEL_HWCONFIG_MAX_NUM_EU_PER_DSS) && len >= 1) {
            devinfo->max_eus_per_subslice = intel_info.hw_config_data[i + 2];
         }
         i += 2 + len;
      }
   }

   if (intel_info.geo_dss_num_bytes > 0 && intel_info.eu_per_dss_mask > 0) {
      uint8_t geo_dss_mask[sizeof(intel_info.geo_dss_mask)] = { 0 };
      uint32_t geo_bytes = intel_info.geo_dss_num_bytes;
      if (geo_bytes > sizeof(geo_dss_mask))
         geo_bytes = sizeof(geo_dss_mask);
      for (uint32_t i = 0; i < geo_bytes; i++)
         geo_dss_mask[i] = (intel_info.geo_dss_mask >> (8 * i)) & 0xff;

      unsigned l3_banks = intel_info.l3_banks ? intel_info.l3_banks : 4;
      intel_device_info_update_dss_topology(devinfo, geo_dss_mask,
                                            geo_bytes,
                                            intel_info.eu_per_dss_mask, l3_banks);
   }

   if (devinfo->subslice_total == 0) {
      if (devinfo->max_slices == 0)
         devinfo->max_slices = 2;
      if (devinfo->max_subslices_per_slice == 0)
         devinfo->max_subslices_per_slice = 4;
      if (devinfo->max_eus_per_subslice == 0)
         devinfo->max_eus_per_subslice = 8;
      if (devinfo->num_thread_per_eu == 0)
         devinfo->num_thread_per_eu = 8;

      uint8_t geo_dss_mask[INTEL_DEVICE_MAX_SLICES * INTEL_DEVICE_MAX_SUBSLICES] = { 0 };
      uint32_t total_dss = devinfo->max_slices * devinfo->max_subslices_per_slice;
      for (uint32_t dss = 0; dss < total_dss && (dss / 8) < sizeof(geo_dss_mask); dss++)
         geo_dss_mask[dss / 8] |= (1u << (dss % 8));

      uint64_t eu_per_dss_mask = (1ull << devinfo->max_eus_per_subslice) - 1;
      unsigned l3_banks = devinfo->l3_banks ? devinfo->l3_banks : 4;
      intel_device_info_update_dss_topology(devinfo, geo_dss_mask,
                                            sizeof(geo_dss_mask),
                                            eu_per_dss_mask, l3_banks);
   }

   if (devinfo->l3_banks == 0)
      devinfo->l3_banks = intel_info.l3_banks ? intel_info.l3_banks : 4;
   if (ts_props.timestamp_frequency > 0)
      devinfo->timestamp_frequency = ts_props.timestamp_frequency;
   else if (devinfo->timestamp_frequency == 0)
      devinfo->timestamp_frequency = (devinfo->ver >= 12) ? 19200000ull : 12000000ull;

   devinfo->has_context_isolation = true;
   devinfo->has_mmap_offset = true;
   devinfo->has_partial_mmap_offset = true;
   devinfo->has_caching_uapi = false;
   devinfo->has_set_pat_uapi = (devinfo->ver >= 20);
   if (intel_info.config_flags & MAGMA_INTEL_CONFIG_FLAG_HAS_VRAM)
      devinfo->has_local_mem = true;
   if (intel_info.config_flags & MAGMA_INTEL_CONFIG_FLAG_HAS_LOW_LATENCY)
      devinfo->supports_low_latency_hint = true;
   if (intel_info.config_flags & MAGMA_INTEL_CONFIG_FLAG_HAS_NO_COMPRESSION_HINT)
      devinfo->xe2_has_no_compression_hint = true;
   if (intel_info.config_flags & MAGMA_INTEL_CONFIG_FLAG_HAS_STATE_CACHE_PERF_FIX)
      devinfo->xe_has_state_cache_perf_fix = true;
   if (intel_info.config_flags & MAGMA_INTEL_CONFIG_FLAG_HAS_PURGING_SUPPORT)
      devinfo->has_madvise_purgeable = true;

   if (props.memory_heap_count > 0) {
      for (uint32_t i = 0; i < props.memory_heap_count; i++) {
         const struct magma_heap *heap = &props.memory_heaps[i];
         if (heap->heap_flags & MAGMA_HEAP_FLAG_DEVICE_LOCAL_BIT) {
            devinfo->has_local_mem = true;
            if (heap->heap_flags & MAGMA_HEAP_FLAG_CPU_VISIBLE_BIT) {
               devinfo->mem.vram.mappable.size = heap->heap_size;
               devinfo->mem.vram.mappable.free = heap->heap_size;
            } else {
               devinfo->mem.vram.unmappable.size = heap->heap_size;
               devinfo->mem.vram.unmappable.free = heap->heap_size;
            }
         } else {
            devinfo->mem.sram.mappable.size = heap->heap_size;
            devinfo->mem.sram.mappable.free = heap->heap_size;
         }
      }
   } else {
      intel_device_info_compute_system_memory(devinfo, false);
   }

   if (as_props.va_size > 0)
      devinfo->gtt_size = as_props.va_size;
   else
      devinfo->gtt_size =
         devinfo->ver >= 8 ? (1ull << 48) : 2ull * 1024 * 1024 * 1024;

   if (as_props.mem_alignment > 0)
      devinfo->mem_alignment = as_props.mem_alignment;
   else
      devinfo->mem_alignment = devinfo->has_local_mem ? (64 * 1024) : 4096;

   for (uint32_t i = 0; i < info.queue_family_count; i++) {
      struct magma_queue_family_properties *qf = &info.queue_families[i];
      uint32_t count = qf->queue_count ? qf->queue_count : 1;

      if (qf->queue_flags & MAGMA_QUEUE_FLAGS_GRAPHICS) {
         devinfo->engine_class_supported_count[INTEL_ENGINE_CLASS_RENDER] += count;
      } else if (qf->queue_flags & MAGMA_QUEUE_FLAGS_COMPUTE) {
         devinfo->engine_class_supported_count[INTEL_ENGINE_CLASS_COMPUTE] += count;
         devinfo->has_compute_engine = true;
      } else if (qf->queue_flags & MAGMA_QUEUE_FLAGS_TRANSFER) {
         devinfo->engine_class_supported_count[INTEL_ENGINE_CLASS_COPY] += count;
      }
   }

   intel_device_info_finish_init(devinfo);

   return true;
}
