/*
 * Copyright 2024 Intel Corporation
 * SPDX-License-Identifier: MIT
 */

#include "intel_perf_common.h"

#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#if defined(MAJOR_IN_SYSMACROS)
#include <sys/sysmacros.h>
#elif defined(MAJOR_IN_MKDEV)
#include <sys/mkdev.h>
#endif

#ifndef HAVE_DIRENT_D_TYPE
#include <limits.h> // PATH_MAX
#endif

#include "dev/intel_debug.h"
#include "dev/intel_device_info.h"
#include "perf/i915/intel_perf.h"
#include "perf/intel_perf.h"
#include "perf/intel_perf_mdapi.h"
#include "perf/intel_perf_metrics.h"
#include "perf/intel_perf_private.h"
#include "perf/intel_perf_regs.h"
#include "perf/xe/intel_perf.h"
#include "util/bitscan.h"
#include "util/log.h"
#include "util/macros.h"
#include "util/mesa-blake3.h"
#include "util/u_debug.h"
#include "util/u_math.h"

#define FILE_DEBUG_FLAG DEBUG_PERFMON

bool
read_file_uint64(const char *file, uint64_t *val)
{
    char buf[32];
    int fd, n;

    fd = open(file, 0);
    if (fd < 0)
       return false;
    while ((n = read(fd, buf, sizeof (buf) - 1)) < 0 &&
           errno == EINTR);
    close(fd);
    if (n < 0)
       return false;

    buf[n] = '\0';
    *val = strtoull(buf, NULL, 0);

    return true;
}

static bool
is_dir_or_link(const struct dirent *entry, const char *parent_dir)
{
#ifdef HAVE_DIRENT_D_TYPE
   return entry->d_type == DT_DIR || entry->d_type == DT_LNK;
#else
   struct stat st;
   char path[PATH_MAX + 1];
   snprintf(path, sizeof(path), "%s/%s", parent_dir, entry->d_name);
   lstat(path, &st);
   return S_ISDIR(st.st_mode) || S_ISLNK(st.st_mode);
#endif
}

static bool
get_sysfs_dev_dir(struct intel_perf_config *perf, int fd)
{
   struct stat sb;
   int min, maj;
   DIR *drmdir;
   struct dirent *drm_entry;
   int len;

   perf->sysfs_dev_dir[0] = '\0';

   if (INTEL_DEBUG(DEBUG_NO_OACONFIG))
      return true;

   if (fstat(fd, &sb)) {
      DBG("Failed to stat DRM fd\n");
      return false;
   }

   maj = major(sb.st_rdev);
   min = minor(sb.st_rdev);

   if (!S_ISCHR(sb.st_mode)) {
      DBG("DRM fd is not a character device as expected\n");
      return false;
   }

   len = snprintf(perf->sysfs_dev_dir,
                  sizeof(perf->sysfs_dev_dir),
                  "/sys/dev/char/%d:%d/device/drm", maj, min);
   if (len < 0 || len >= sizeof(perf->sysfs_dev_dir)) {
      DBG("Failed to concatenate sysfs path to drm device\n");
      return false;
   }

   drmdir = opendir(perf->sysfs_dev_dir);
   if (!drmdir) {
      DBG("Failed to open %s: %m\n", perf->sysfs_dev_dir);
      return false;
   }

   while ((drm_entry = readdir(drmdir))) {
      if (is_dir_or_link(drm_entry, perf->sysfs_dev_dir) &&
          strncmp(drm_entry->d_name, "card", 4) == 0)
      {
         len = snprintf(perf->sysfs_dev_dir,
                        sizeof(perf->sysfs_dev_dir),
                        "/sys/dev/char/%d:%d/device/drm/%s",
                        maj, min, drm_entry->d_name);
         closedir(drmdir);
         if (len < 0 || len >= sizeof(perf->sysfs_dev_dir))
            return false;
         else
            return true;
      }
   }

   closedir(drmdir);

   DBG("Failed to find cardX directory under /sys/dev/char/%d:%d/device/drm\n",
       maj, min);

   return false;
}

static bool
read_sysfs_drm_device_file_uint64(struct intel_perf_config *perf,
                                  const char *file,
                                  uint64_t *value)
{
   char buf[512];
   int len;

   len = snprintf(buf, sizeof(buf), "%s/%s", perf->sysfs_dev_dir, file);
   if (len < 0 || len >= sizeof(buf)) {
      DBG("Failed to concatenate sys filename to read u64 from\n");
      return false;
   }

   return read_file_uint64(buf, value);
}

static bool
oa_config_enabled(struct intel_perf_config *perf,
                  const struct intel_perf_query_info *query) {
   // Hide extended metrics unless enabled with env param
   bool is_extended_metric = strncmp(query->name, "Ext", 3) == 0;

   return perf->enable_all_metrics || !is_extended_metric;
}

static void
register_oa_config(struct intel_perf_config *perf,
                   const struct intel_device_info *devinfo,
                   const struct intel_perf_query_info *query,
                   uint64_t config_id)
{
   if (!oa_config_enabled(perf, query))
      return;

   /* oa format handling needs to match with platform version returned in
    * intel_perf_get_oa_format()
    */
   assert(intel_perf_get_oa_format(perf) == query->oa_format);

   struct intel_perf_query_info *registered_query =
      intel_perf_append_query_info(perf, 0);

   *registered_query = *query;
   registered_query->oa_metrics_set_id = config_id;
   DBG("metric set registered: id = %" PRIu64", guid = %s\n",
       registered_query->oa_metrics_set_id, query->guid);
}

static void
enumerate_sysfs_metrics(struct intel_perf_config *perf,
                        const struct intel_device_info *devinfo)
{
   DIR *metricsdir = NULL;
   struct dirent *metric_entry;
   char buf[256];
   int len;

   len = snprintf(buf, sizeof(buf), "%s/metrics", perf->sysfs_dev_dir);
   if (len < 0 || len >= sizeof(buf)) {
      mesa_logw("intel_perf: failed to concatenate path to sysfs metrics/ directory\n");
      return;
   }

   metricsdir = opendir(buf);
   if (!metricsdir) {
      mesa_logw("intel_perf: failed to open OA metrics directory %s: %m\n", buf);
      return;
   }

   while ((metric_entry = readdir(metricsdir))) {
      struct hash_entry *entry;
      if (!is_dir_or_link(metric_entry, buf) ||
          metric_entry->d_name[0] == '.')
         continue;

      DBG("metric set: %s\n", metric_entry->d_name);
      entry = _mesa_hash_table_search(perf->oa_metrics_table,
                                      metric_entry->d_name);
      if (entry) {
         uint64_t id;
         if (!intel_perf_load_metric_id(perf, metric_entry->d_name, &id)) {
            DBG("Failed to read metric set id from %s: %m", buf);
            continue;
         }

         register_oa_config(perf, devinfo,
                            (const struct intel_perf_query_info *)entry->data, id);
      } else
         DBG("metric set not known by mesa (skipping)\n");
   }

   closedir(metricsdir);
}

static void
add_all_metrics(struct intel_perf_config *perf,
                const struct intel_device_info *devinfo)
{
   hash_table_foreach(perf->oa_metrics_table, entry) {
      const struct intel_perf_query_info *query = entry->data;
      register_oa_config(perf, devinfo, query, 0);
   }
}

static bool
kernel_has_dynamic_config_support(struct intel_perf_config *perf, int fd)
{
   switch (perf->devinfo->kmd_type) {
   case INTEL_KMD_TYPE_I915:
      return i915_has_dynamic_config_support(perf, fd);
   case INTEL_KMD_TYPE_XE:
      return true;
   default:
      UNREACHABLE("missing");
      return false;
   }
}

bool
intel_perf_load_metric_id(struct intel_perf_config *perf_cfg,
                          const char *guid,
                          uint64_t *metric_id)
{
   char config_path[280];

   snprintf(config_path, sizeof(config_path), "%s/metrics/%s/id",
            perf_cfg->sysfs_dev_dir, guid);

   /* Don't recreate already loaded configs. */
   return read_file_uint64(config_path, metric_id);
}

static uint64_t
kmd_add_config(struct intel_perf_config *perf, int fd,
               const struct intel_perf_registers *config,
               const char *guid)
{
   switch (perf->devinfo->kmd_type) {
   case INTEL_KMD_TYPE_I915:
      return i915_add_config(perf, fd, config, guid);
   case INTEL_KMD_TYPE_XE:
      return xe_add_config(perf, fd, config, guid);
   default:
      UNREACHABLE("missing");
      return 0;
   }
}

static void
init_oa_configs(struct intel_perf_config *perf, int fd,
                const struct intel_device_info *devinfo)
{
   hash_table_foreach(perf->oa_metrics_table, entry) {
      const struct intel_perf_query_info *query = entry->data;
      uint64_t config_id;

      if (intel_perf_load_metric_id(perf, query->guid, &config_id)) {
         DBG("metric set: %s (already loaded)\n", query->guid);
         register_oa_config(perf, devinfo, query, config_id);
         continue;
      }

      uint64_t ret = kmd_add_config(perf, fd, &query->config, query->guid);
      if (ret == 0) {
         DBG("Failed to load \"%s\" (%s) metrics set in kernel: %s\n",
             query->name, query->guid, strerror(errno));
         continue;
      }

      register_oa_config(perf, devinfo, query, ret);
      DBG("metric set: %s (added)\n", query->guid);
   }
}

static void
compute_topology_builtins(struct intel_perf_config *perf)
{
   const struct intel_device_info *devinfo = perf->devinfo;

   perf->sys_vars.slice_mask = devinfo->slice_masks;
   perf->sys_vars.n_eu_slices = devinfo->num_slices;
   perf->sys_vars.n_l3_banks = devinfo->l3_banks;
   perf->sys_vars.n_l3_nodes = devinfo->l3_banks / 4;
   perf->sys_vars.n_sq_idis =  devinfo->num_slices;
   perf->sys_vars.n_depth_pipes = devinfo->num_depth_pipes;
   perf->sys_vars.n_geom_pipes = devinfo->num_geom_pipes;
   perf->sys_vars.n_color_pipes = devinfo->num_color_pipes;

   perf->sys_vars.n_eu_slice0123 = 0;
   for (int s = 0; s < MIN2(4, devinfo->max_slices); s++) {
      if (!intel_device_info_slice_available(devinfo, s))
         continue;

      for (int ss = 0; ss < devinfo->max_subslices_per_slice; ss++) {
         if (!intel_device_info_subslice_available(devinfo, s, ss))
            continue;

         for (int eu = 0; eu < devinfo->max_eus_per_subslice; eu++) {
            if (intel_device_info_eu_available(devinfo, s, ss, eu))
               perf->sys_vars.n_eu_slice0123++;
         }
      }
   }

   perf->sys_vars.n_eu_sub_slices = intel_device_info_subslice_total(devinfo);
   perf->sys_vars.n_eus = intel_device_info_eu_total(devinfo);

   /* The subslice mask builtin contains bits for all slices. Prior to Gfx11
    * it had groups of 3bits for each slice, on Gfx11 and above it's 8bits for
    * each slice.
    *
    * Ideally equations would be updated to have a slice/subslice query
    * function/operator.
    */
   perf->sys_vars.subslice_mask = 0;

   int bits_per_subslice = devinfo->ver >= 11 ? 8 : 3;

   for (int s = 0; s < util_last_bit(devinfo->slice_masks); s++) {
      for (int ss = 0; ss < (devinfo->subslice_slice_stride * 8); ss++) {
         if (intel_device_info_subslice_available(devinfo, s, ss))
            perf->sys_vars.subslice_mask |= 1ULL << (s * bits_per_subslice + ss);
      }
   }
}

static bool
init_oa_sys_vars(struct intel_perf_config *perf,
                 bool use_register_snapshots)
{
   uint64_t min_freq_mhz = 0, max_freq_mhz = 0;

   if (!INTEL_DEBUG(DEBUG_NO_OACONFIG)) {
      const char *min_file, *max_file;

      switch (perf->devinfo->kmd_type) {
      case INTEL_KMD_TYPE_I915:
         min_file = "gt_min_freq_mhz";
         max_file = "gt_max_freq_mhz";
         break;
      case INTEL_KMD_TYPE_XE:
         min_file = "device/tile0/gt0/freq0/min_freq";
         max_file = "device/tile0/gt0/freq0/max_freq";
         break;
      default:
         UNREACHABLE("missing");
         return false;
      }

      if (!read_sysfs_drm_device_file_uint64(perf, min_file, &min_freq_mhz))
         return false;

      if (!read_sysfs_drm_device_file_uint64(perf, max_file, &max_freq_mhz))
         return false;
   } else {
      min_freq_mhz = 300;
      max_freq_mhz = 1000;
   }

   memset(&perf->sys_vars, 0, sizeof(perf->sys_vars));
   perf->sys_vars.gt_min_freq = min_freq_mhz * 1000000;
   perf->sys_vars.gt_max_freq = max_freq_mhz * 1000000;
   perf->sys_vars.query_mode = use_register_snapshots;
   compute_topology_builtins(perf);

   return true;
}

typedef void (*perf_register_oa_queries_t)(struct intel_perf_config *);

static perf_register_oa_queries_t
get_register_queries_function(const struct intel_device_info *devinfo)
{
   switch (devinfo->platform) {
   case INTEL_PLATFORM_HSW:
      return intel_oa_register_queries_hsw;
   case INTEL_PLATFORM_CHV:
      return intel_oa_register_queries_chv;
   case INTEL_PLATFORM_BDW:
      return intel_oa_register_queries_bdw;
   case INTEL_PLATFORM_BXT:
      return intel_oa_register_queries_bxt;
   case INTEL_PLATFORM_SKL:
      if (devinfo->gt == 2)
         return intel_oa_register_queries_sklgt2;
      if (devinfo->gt == 3)
         return intel_oa_register_queries_sklgt3;
      if (devinfo->gt == 4)
         return intel_oa_register_queries_sklgt4;
      return NULL;
   case INTEL_PLATFORM_KBL:
      if (devinfo->gt == 2)
         return intel_oa_register_queries_kblgt2;
      if (devinfo->gt == 3)
         return intel_oa_register_queries_kblgt3;
      return NULL;
   case INTEL_PLATFORM_GLK:
      return intel_oa_register_queries_glk;
   case INTEL_PLATFORM_CFL:
      if (devinfo->gt == 2)
         return intel_oa_register_queries_cflgt2;
      if (devinfo->gt == 3)
         return intel_oa_register_queries_cflgt3;
      return NULL;
   case INTEL_PLATFORM_ICL:
      return intel_oa_register_queries_icl;
   case INTEL_PLATFORM_EHL:
      return intel_oa_register_queries_ehl;
   case INTEL_PLATFORM_TGL:
      if (devinfo->gt == 1)
         return intel_oa_register_queries_tglgt1;
      if (devinfo->gt == 2)
         return intel_oa_register_queries_tglgt2;
      return NULL;
   case INTEL_PLATFORM_RKL:
      return intel_oa_register_queries_rkl;
   case INTEL_PLATFORM_DG1:
      return intel_oa_register_queries_dg1;
   case INTEL_PLATFORM_ADL:
   case INTEL_PLATFORM_RPL:
      return intel_oa_register_queries_adl;
   case INTEL_PLATFORM_DG2_G10:
      return intel_oa_register_queries_acmgt3;
   case INTEL_PLATFORM_DG2_G11:
      return intel_oa_register_queries_acmgt1;
   case INTEL_PLATFORM_DG2_G12:
      return intel_oa_register_queries_acmgt2;
   case INTEL_PLATFORM_MTL_U:
   case INTEL_PLATFORM_MTL_H:
      if (intel_device_info_eu_total(devinfo) <= 64)
         return intel_oa_register_queries_mtlgt2;
      if (intel_device_info_eu_total(devinfo) <= 128)
         return intel_oa_register_queries_mtlgt3;
      return NULL;
   case INTEL_PLATFORM_ARL_U:
   case INTEL_PLATFORM_ARL_H:
      if (intel_device_info_eu_total(devinfo) <= 64)
         return intel_oa_register_queries_arlgt1;
      if (intel_device_info_eu_total(devinfo) <= 128)
         return intel_oa_register_queries_arlgt2;
      return NULL;
   case INTEL_PLATFORM_LNL:
      return intel_oa_register_queries_lnl;
   case INTEL_PLATFORM_BMG:
      return intel_oa_register_queries_bmg;
   case INTEL_PLATFORM_PTL:
   case INTEL_PLATFORM_WCL:
      return intel_oa_register_queries_ptl;
   default:
      return NULL;
   }
}

static int
intel_perf_compare_counter_names(const void *v1, const void *v2)
{
   const struct intel_perf_query_counter *c1 = v1;
   const struct intel_perf_query_counter *c2 = v2;

   return strcmp(c1->name, c2->name);
}

static void
sort_query(struct intel_perf_query_info *q)
{
   qsort(q->counters, q->n_counters, sizeof(q->counters[0]),
         intel_perf_compare_counter_names);
}

static void
load_pipeline_statistic_metrics(struct intel_perf_config *perf_cfg,
                                const struct intel_device_info *devinfo)
{
   struct intel_perf_query_info *query =
      intel_perf_append_query_info(perf_cfg, MAX_STAT_COUNTERS);

   query->kind = INTEL_PERF_QUERY_TYPE_PIPELINE;
   query->name = "Pipeline Statistics Registers";

   intel_perf_query_add_basic_stat_reg(query, IA_VERTICES_COUNT,
                                       "N vertices submitted");
   intel_perf_query_add_basic_stat_reg(query, IA_PRIMITIVES_COUNT,
                                       "N primitives submitted");
   intel_perf_query_add_basic_stat_reg(query, VS_INVOCATION_COUNT,
                                       "N vertex shader invocations");

   if (devinfo->ver == 6) {
      intel_perf_query_add_stat_reg(query, GFX6_SO_PRIM_STORAGE_NEEDED, 1, 1,
                                    "SO_PRIM_STORAGE_NEEDED",
                                    "N geometry shader stream-out primitives (total)");
      intel_perf_query_add_stat_reg(query, GFX6_SO_NUM_PRIMS_WRITTEN, 1, 1,
                                    "SO_NUM_PRIMS_WRITTEN",
                                    "N geometry shader stream-out primitives (written)");
   } else {
      intel_perf_query_add_stat_reg(query, GFX7_SO_PRIM_STORAGE_NEEDED(0), 1, 1,
                                    "SO_PRIM_STORAGE_NEEDED (Stream 0)",
                                    "N stream-out (stream 0) primitives (total)");
      intel_perf_query_add_stat_reg(query, GFX7_SO_PRIM_STORAGE_NEEDED(1), 1, 1,
                                    "SO_PRIM_STORAGE_NEEDED (Stream 1)",
                                    "N stream-out (stream 1) primitives (total)");
      intel_perf_query_add_stat_reg(query, GFX7_SO_PRIM_STORAGE_NEEDED(2), 1, 1,
                                    "SO_PRIM_STORAGE_NEEDED (Stream 2)",
                                    "N stream-out (stream 2) primitives (total)");
      intel_perf_query_add_stat_reg(query, GFX7_SO_PRIM_STORAGE_NEEDED(3), 1, 1,
                                    "SO_PRIM_STORAGE_NEEDED (Stream 3)",
                                    "N stream-out (stream 3) primitives (total)");
      intel_perf_query_add_stat_reg(query, GFX7_SO_NUM_PRIMS_WRITTEN(0), 1, 1,
                                    "SO_NUM_PRIMS_WRITTEN (Stream 0)",
                                    "N stream-out (stream 0) primitives (written)");
      intel_perf_query_add_stat_reg(query, GFX7_SO_NUM_PRIMS_WRITTEN(1), 1, 1,
                                    "SO_NUM_PRIMS_WRITTEN (Stream 1)",
                                    "N stream-out (stream 1) primitives (written)");
      intel_perf_query_add_stat_reg(query, GFX7_SO_NUM_PRIMS_WRITTEN(2), 1, 1,
                                    "SO_NUM_PRIMS_WRITTEN (Stream 2)",
                                    "N stream-out (stream 2) primitives (written)");
      intel_perf_query_add_stat_reg(query, GFX7_SO_NUM_PRIMS_WRITTEN(3), 1, 1,
                                    "SO_NUM_PRIMS_WRITTEN (Stream 3)",
                                    "N stream-out (stream 3) primitives (written)");
   }

   intel_perf_query_add_basic_stat_reg(query, HS_INVOCATION_COUNT,
                                       "N TCS shader invocations");
   intel_perf_query_add_basic_stat_reg(query, DS_INVOCATION_COUNT,
                                       "N TES shader invocations");

   intel_perf_query_add_basic_stat_reg(query, GS_INVOCATION_COUNT,
                                       "N geometry shader invocations");
   intel_perf_query_add_basic_stat_reg(query, GS_PRIMITIVES_COUNT,
                                       "N geometry shader primitives emitted");

   intel_perf_query_add_basic_stat_reg(query, CL_INVOCATION_COUNT,
                                       "N primitives entering clipping");
   intel_perf_query_add_basic_stat_reg(query, CL_PRIMITIVES_COUNT,
                                       "N primitives leaving clipping");

   if (devinfo->verx10 == 75 || devinfo->ver == 8) {
      intel_perf_query_add_stat_reg(query, PS_INVOCATION_COUNT, 1, 4,
                                    "N fragment shader invocations",
                                    "N fragment shader invocations");
   } else {
      intel_perf_query_add_basic_stat_reg(query, PS_INVOCATION_COUNT,
                                          "N fragment shader invocations");
   }

   intel_perf_query_add_basic_stat_reg(query, PS_DEPTH_COUNT,
                                       "N z-pass fragments");

   if (devinfo->ver >= 7) {
      intel_perf_query_add_basic_stat_reg(query, CS_INVOCATION_COUNT,
                                          "N compute shader invocations");
   }

   query->data_size = sizeof(uint64_t) * query->n_counters;

   sort_query(query);
}

static inline int
compare_str_or_null(const char *s1, const char *s2)
{
   if (s1 == NULL && s2 == NULL)
      return 0;
   if (s1 == NULL)
      return -1;
   if (s2 == NULL)
      return 1;

   return strcmp(s1, s2);
}

static int
compare_counter_categories_and_names(const void *_c1, const void *_c2)
{
   const struct intel_perf_query_counter_info *c1 = (const struct intel_perf_query_counter_info *)_c1;
   const struct intel_perf_query_counter_info *c2 = (const struct intel_perf_query_counter_info *)_c2;

   /* pipeline counters don't have an assigned category */
   int r = compare_str_or_null(c1->counter->category, c2->counter->category);
   if (r)
      return r;

   return strcmp(c1->counter->name, c2->counter->name);
}

static void
build_unique_counter_list(struct intel_perf_config *perf)
{
   size_t max_counters = 0;

   for (int q = 0; q < perf->n_queries; q++)
      max_counters += perf->queries[q].n_counters;

   /*
    * Allocate big enough array to hold maximum possible number of counters.
    * We can't alloc it small and realloc when needed because the hash table
    * below contains pointers to this array.
    */
   struct intel_perf_query_counter_info *counter_infos =
         rzalloc_array_size(perf, sizeof(counter_infos[0]), max_counters);

   perf->n_counters = 0;

   struct hash_table *counters_table =
      _mesa_hash_table_create(NULL,
                              _mesa_hash_string,
                              _mesa_key_string_equal);
   struct hash_entry *entry;
   for (int q = 0; q < perf->n_queries ; q++) {
      struct intel_perf_query_info *query = &perf->queries[q];

      for (int c = 0; c < query->n_counters; c++) {
         struct intel_perf_query_counter *counter;
         struct intel_perf_query_counter_info *counter_info;

         counter = &query->counters[c];
         entry = _mesa_hash_table_search(counters_table, counter->symbol_name);

         if (entry) {
            counter_info = entry->data;
            BITSET_SET(counter_info->query_mask, q);
            continue;
         }
         assert(perf->n_counters < max_counters);

         counter_info = &counter_infos[perf->n_counters++];
         counter_info->counter = counter;
         BITSET_SET(counter_info->query_mask, q);

         counter_info->location.group_idx = q;
         counter_info->location.counter_idx = c;

         _mesa_hash_table_insert(counters_table, counter->symbol_name, counter_info);
      }
   }

   _mesa_hash_table_destroy(counters_table, NULL);

   perf->counter_infos = counter_infos;

   qsort(perf->counter_infos, perf->n_counters, sizeof(perf->counter_infos[0]),
         compare_counter_categories_and_names);
}

static bool
oa_metrics_available(struct intel_perf_config *perf, int fd,
                     const struct intel_device_info *devinfo,
                     bool use_register_snapshots)
{
   perf_register_oa_queries_t oa_register = get_register_queries_function(devinfo);
   bool oa_metrics_available = false;

   /* TODO: Support performance metrics */
   if (devinfo->is_virtio)
      return false;

   perf->devinfo = devinfo;

   /* Consider an invalid as supported. */
   if (fd == -1)
      return true;

   perf->enable_all_metrics = debug_get_bool_option("INTEL_EXTENDED_METRICS", false);

   /* TODO: We should query this from i915?
    * Looks like Xe2 platforms don't need it but don't have a spec quote to
    * back it.
    */
   if (devinfo->verx10 == 125)
      perf->oa_timestamp_shift = 1;

   perf->oa_timestamp_mask =
      0xffffffffffffffffull >> (32 + perf->oa_timestamp_shift);

   switch (devinfo->kmd_type) {
   case INTEL_KMD_TYPE_I915:
      oa_metrics_available = i915_oa_metrics_available(perf, fd, use_register_snapshots);
      break;
   case INTEL_KMD_TYPE_XE:
      oa_metrics_available = xe_oa_metrics_available(perf, fd, use_register_snapshots);
      break;
   default:
      UNREACHABLE("missing");
      break;
   }

   return oa_metrics_available &&
          oa_register &&
          get_sysfs_dev_dir(perf, fd) &&
          init_oa_sys_vars(perf, use_register_snapshots);
}

static void
load_oa_metrics(struct intel_perf_config *perf, int fd,
                const struct intel_device_info *devinfo)
{
   int existing_queries = perf->n_queries;

   perf_register_oa_queries_t oa_register = get_register_queries_function(devinfo);

   perf->oa_metrics_table =
      _mesa_hash_table_create(perf, _mesa_hash_string,
                              _mesa_key_string_equal);

   /* Index all the metric sets mesa knows about before looking to see what
    * the kernel is advertising.
    */
   oa_register(perf);

   if (!INTEL_DEBUG(DEBUG_NO_OACONFIG)) {
      if (kernel_has_dynamic_config_support(perf, fd))
         init_oa_configs(perf, fd, devinfo);
      else
         enumerate_sysfs_metrics(perf, devinfo);
   } else {
      add_all_metrics(perf, devinfo);
   }

   /* sort counters in each individual group created by this function by name */
   for (int i = existing_queries; i < perf->n_queries; ++i)
      sort_query(&perf->queries[i]);

   /* Select a fallback OA metric. Look for the TestOa metric or use the last
    * one if no present (on HSW).
    */
   for (int i = existing_queries; i < perf->n_queries; i++) {
      if (perf->queries[i].symbol_name &&
          strcmp(perf->queries[i].symbol_name, "TestOa") == 0) {
         perf->fallback_raw_oa_metric = perf->queries[i].oa_metrics_set_id;
         break;
      }
   }
   if (perf->fallback_raw_oa_metric == 0 && perf->n_queries > 0)
      perf->fallback_raw_oa_metric = perf->queries[perf->n_queries - 1].oa_metrics_set_id;
}

uint64_t
intel_perf_get_configuration_id(struct intel_perf_config *perf_cfg, const char *guid)
{
   char path[512];
   uint64_t val;

   snprintf(path, sizeof(path), "metrics/%s/id", guid);
   if (read_sysfs_drm_device_file_uint64(perf_cfg, path, &val))
      return val;

   return 0;
}

uint64_t
intel_perf_store_configuration(struct intel_perf_config *perf_cfg, int fd,
                               const struct intel_perf_registers *config,
                               const char *guid)
{
   if (guid)
      return kmd_add_config(perf_cfg, fd, config, guid);

   blake3_hasher blake3_ctx;
   _mesa_blake3_init(&blake3_ctx);

   if (config->flex_regs) {
      _mesa_blake3_update(&blake3_ctx, config->flex_regs,
                        sizeof(config->flex_regs[0]) *
                        config->n_flex_regs);
   }
   if (config->mux_regs) {
      _mesa_blake3_update(&blake3_ctx, config->mux_regs,
                        sizeof(config->mux_regs[0]) *
                        config->n_mux_regs);
   }
   if (config->b_counter_regs) {
      _mesa_blake3_update(&blake3_ctx, config->b_counter_regs,
                        sizeof(config->b_counter_regs[0]) *
                        config->n_b_counter_regs);
   }

   uint8_t hash[BLAKE3_KEY_LEN];
   _mesa_blake3_final(&blake3_ctx, hash);

   char formatted_hash[BLAKE3_HEX_LEN];
   _mesa_blake3_format(formatted_hash, hash);

   char generated_guid[37];
   snprintf(generated_guid, sizeof(generated_guid),
            "%.8s-%.4s-%.4s-%.4s-%.12s",
            &formatted_hash[0], &formatted_hash[8],
            &formatted_hash[8 + 4], &formatted_hash[8 + 4 + 4],
            &formatted_hash[8 + 4 + 4 + 4]);

   /* Check if already present. */
   uint64_t id;
   if (intel_perf_load_metric_id(perf_cfg, generated_guid, &id))
      return id;

   return kmd_add_config(perf_cfg, fd, config, generated_guid);
}

void
intel_perf_remove_configuration(struct intel_perf_config *perf_cfg, int fd,
                                uint64_t config_id)
{
   switch (perf_cfg->devinfo->kmd_type) {
   case INTEL_KMD_TYPE_I915:
      i915_remove_config(perf_cfg, fd, config_id);
      break;
   case INTEL_KMD_TYPE_XE:
      xe_remove_config(perf_cfg, fd, config_id);
      break;
   default:
      UNREACHABLE("missing");
   }
}

static int
intel_perf_compare_query_names(const void *v1, const void *v2)
{
   const struct intel_perf_query_info *q1 = v1;
   const struct intel_perf_query_info *q2 = v2;

   return strcmp(q1->name, q2->name);
}

/* Xe2: (64 x PEC) + SRM_RPSTAT + MI_RPC */
#define MAX_QUERY_FIELDS(devinfo) (devinfo->verx10 >= 200 ? (64 + 2) : (5 + 16))

static inline struct intel_perf_query_field *
add_query_register(struct intel_perf_config *perf_cfg,
                   enum intel_perf_query_field_type type,
                   uint32_t offset,
                   uint16_t size,
                   uint8_t index)
{
   struct intel_perf_query_field_layout *layout = &perf_cfg->query_layout;

   /* Align MI_RPC to 64bytes (HW requirement) & 64bit registers to 8bytes
    * (shows up nicely in the debugger).
    */
   if (type == INTEL_PERF_QUERY_FIELD_TYPE_MI_RPC)
      layout->size = align(layout->size, 64);
   else if (size % 8 == 0)
      layout->size = align(layout->size, 8);

   assert(layout->n_fields < MAX_QUERY_FIELDS(perf_cfg->devinfo));
   layout->fields[layout->n_fields++] = (struct intel_perf_query_field) {
      .mmio_offset = offset,
      .location = layout->size,
      .type = type,
      .index = index,
      .size = size,
   };
   layout->size += size;

   return &layout->fields[layout->n_fields - 1];
}

static void
intel_perf_init_query_fields(struct intel_perf_config *perf_cfg,
                             const struct intel_device_info *devinfo,
                             bool use_register_snapshots)
{
   struct intel_perf_query_field_layout *layout = &perf_cfg->query_layout;

   layout->n_fields = 0;

   /* MI_RPC requires a 64byte alignment. */
   layout->alignment = 64;

   layout->fields = rzalloc_array(perf_cfg, struct intel_perf_query_field,
                                  MAX_QUERY_FIELDS(devinfo));

   add_query_register(perf_cfg, INTEL_PERF_QUERY_FIELD_TYPE_MI_RPC,
                      0, perf_cfg->oa_sample_size, 0);

   if (use_register_snapshots) {
      if (devinfo->ver <= 11) {
         struct intel_perf_query_field *field =
            add_query_register(perf_cfg,
                               INTEL_PERF_QUERY_FIELD_TYPE_SRM_PERFCNT,
                               PERF_CNT_1_DW0, 8, 0);
         field->mask = PERF_CNT_VALUE_MASK;

         field = add_query_register(perf_cfg,
                                    INTEL_PERF_QUERY_FIELD_TYPE_SRM_PERFCNT,
                                    PERF_CNT_2_DW0, 8, 1);
         field->mask = PERF_CNT_VALUE_MASK;
      }

      if (devinfo->ver == 8 && devinfo->platform != INTEL_PLATFORM_CHV) {
         add_query_register(perf_cfg,
                            INTEL_PERF_QUERY_FIELD_TYPE_SRM_RPSTAT,
                            GFX7_RPSTAT1, 4, 0);
      }

      if (devinfo->ver >= 9) {
         add_query_register(perf_cfg,
                            INTEL_PERF_QUERY_FIELD_TYPE_SRM_RPSTAT,
                            GFX9_RPSTAT0, 4, 0);
      }

      if (!can_use_mi_rpc_bc_counters(devinfo)) {
         if (devinfo->ver >= 8 && devinfo->ver <= 11) {
            for (uint32_t i = 0; i < GFX8_N_OA_PERF_B32; i++) {
               add_query_register(perf_cfg, INTEL_PERF_QUERY_FIELD_TYPE_SRM_OA_B,
                                  GFX8_OA_PERF_B32(i), 4, i);
            }
            for (uint32_t i = 0; i < GFX8_N_OA_PERF_C32; i++) {
               add_query_register(perf_cfg, INTEL_PERF_QUERY_FIELD_TYPE_SRM_OA_C,
                                  GFX8_OA_PERF_C32(i), 4, i);
            }
         } else if (devinfo->verx10 == 120) {
            for (uint32_t i = 0; i < GFX12_N_OAG_PERF_B32; i++) {
               add_query_register(perf_cfg, INTEL_PERF_QUERY_FIELD_TYPE_SRM_OA_B,
                                  GFX12_OAG_PERF_B32(i), 4, i);
            }
            for (uint32_t i = 0; i < GFX12_N_OAG_PERF_C32; i++) {
               add_query_register(perf_cfg, INTEL_PERF_QUERY_FIELD_TYPE_SRM_OA_C,
                                  GFX12_OAG_PERF_C32(i), 4, i);
            }
         } else if (devinfo->verx10 == 125) {
            add_query_register(perf_cfg, INTEL_PERF_QUERY_FIELD_TYPE_SRM_OA_A,
                               GFX125_OAG_PERF_A36, 4, 36);
            add_query_register(perf_cfg, INTEL_PERF_QUERY_FIELD_TYPE_SRM_OA_A,
                               GFX125_OAG_PERF_A37, 4, 37);
            for (uint32_t i = 0; i < GFX12_N_OAG_PERF_B32; i++) {
               add_query_register(perf_cfg, INTEL_PERF_QUERY_FIELD_TYPE_SRM_OA_B,
                                  GFX12_OAG_PERF_B32(i), 4, i);
            }
            for (uint32_t i = 0; i < GFX12_N_OAG_PERF_C32; i++) {
               add_query_register(perf_cfg, INTEL_PERF_QUERY_FIELD_TYPE_SRM_OA_C,
                                  GFX12_OAG_PERF_C32(i), 4, i);
            }
         }
      }
   }

   /* Align the whole package to 64bytes so that 2 snapshots can be put
    * together without extract alignment for the user.
    */
   layout->size = align(layout->size, 64);
}

static size_t
intel_perf_get_oa_format_size(const struct intel_device_info *devinfo)
{
   if (devinfo->verx10 >= 200)
      return 576;

   return 256;
}

void
intel_perf_init_metrics(struct intel_perf_config *perf_cfg,
                        const struct intel_device_info *devinfo,
                        int drm_fd,
                        bool include_pipeline_statistics,
                        bool use_register_snapshots)
{
   perf_cfg->devinfo = devinfo;
   perf_cfg->oa_sample_size = intel_perf_get_oa_format_size(devinfo);
   perf_cfg->use_metrics_library = debug_get_bool_option("INTEL_USE_METRICS_LIBRARY", false);

   intel_perf_init_query_fields(perf_cfg, devinfo, use_register_snapshots);

   if (include_pipeline_statistics) {
      load_pipeline_statistic_metrics(perf_cfg, devinfo);
      intel_perf_register_mdapi_statistic_query(perf_cfg, devinfo);
   }

   bool oa_metrics = oa_metrics_available(perf_cfg, drm_fd, devinfo,
                                          use_register_snapshots);
   if (oa_metrics)
      load_oa_metrics(perf_cfg, drm_fd, devinfo);

   /* sort query groups by name */
   if (perf_cfg->queries != NULL) {
      qsort(perf_cfg->queries, perf_cfg->n_queries,
            sizeof(perf_cfg->queries[0]), intel_perf_compare_query_names);
   }

   build_unique_counter_list(perf_cfg);

   if (oa_metrics)
      intel_perf_register_mdapi_oa_query(perf_cfg, devinfo);
}

uint64_t
intel_perf_get_oa_format(struct intel_perf_config *perf_cfg)
{
   switch (perf_cfg->devinfo->kmd_type) {
   case INTEL_KMD_TYPE_I915:
      return i915_perf_get_oa_format(perf_cfg);
   case INTEL_KMD_TYPE_XE:
      return xe_perf_get_oa_format(perf_cfg);
   default:
      UNREACHABLE("missing");
      return 0;
   }
}

int
intel_perf_stream_open(struct intel_perf_config *perf_config, int drm_fd,
                       uint32_t ctx_id, uint64_t metrics_set_id,
                       uint64_t period_exponent, bool hold_preemption,
                       bool enable, struct intel_bind_timeline *timeline)
{
   uint64_t report_format = intel_perf_get_oa_format(perf_config);

   switch (perf_config->devinfo->kmd_type) {
   case INTEL_KMD_TYPE_I915:
      return i915_perf_stream_open(perf_config, drm_fd, ctx_id, metrics_set_id,
                                   report_format, period_exponent,
                                   hold_preemption, enable);
   case INTEL_KMD_TYPE_XE:
      return xe_perf_stream_open(perf_config, drm_fd, ctx_id, metrics_set_id,
                                 report_format, period_exponent,
                                 hold_preemption, enable, timeline);
   default:
         UNREACHABLE("missing");
         return 0;
   }
}

/*
 * Read perf stream samples.
 *
 * buffer will be filled with multiple struct intel_perf_record_header + data.
 *
 * Returns 0 if no sample is available, -errno value if a error happened or
 * the number of bytes read on success.
 */
int
intel_perf_stream_read_samples(struct intel_perf_config *perf_config,
                               int perf_stream_fd, uint8_t *buffer,
                               size_t buffer_len)
{
   switch (perf_config->devinfo->kmd_type) {
   case INTEL_KMD_TYPE_I915:
      return i915_perf_stream_read_samples(perf_config, perf_stream_fd, buffer, buffer_len);
   case INTEL_KMD_TYPE_XE:
      return xe_perf_stream_read_samples(perf_config, perf_stream_fd, buffer, buffer_len);
   default:
         UNREACHABLE("missing");
         return -1;
   }
}

int
intel_perf_stream_set_state(struct intel_perf_config *perf_config,
                            int perf_stream_fd, bool enable)
{
   switch (perf_config->devinfo->kmd_type) {
   case INTEL_KMD_TYPE_I915:
      return i915_perf_stream_set_state(perf_stream_fd, enable);
   case INTEL_KMD_TYPE_XE:
      return xe_perf_stream_set_state(perf_stream_fd, enable);
   default:
         UNREACHABLE("missing");
         return -1;
   }
}

void *
intel_perf_stream_map_oa_buffer(struct intel_perf_config *perf_config,
                                int perf_stream_fd, uint64_t *size)
{
   switch (perf_config->devinfo->kmd_type) {
   case INTEL_KMD_TYPE_XE:
      return xe_perf_stream_map_oa_buffer(perf_stream_fd, size);
   default:
      /* i915 has no uapi to map the OA buffer. */
      return NULL;
   }
}

void
intel_perf_stream_unmap_oa_buffer(void *map, uint64_t size)
{
   if (map)
      munmap(map, size);
}

int
intel_perf_stream_set_metrics_id(struct intel_perf_config *perf_config,
                                 int drm_fd, int perf_stream_fd,
                                 uint32_t exec_queue,
                                 uint64_t metrics_set_id,
                                 struct intel_bind_timeline *timeline)
{
   switch (perf_config->devinfo->kmd_type) {
   case INTEL_KMD_TYPE_I915:
      return i915_perf_stream_set_metrics_id(perf_stream_fd, metrics_set_id);
   case INTEL_KMD_TYPE_XE:
      return xe_perf_stream_set_metrics_id(perf_stream_fd, drm_fd,
                                           exec_queue, metrics_set_id,
                                           timeline);
   default:
         UNREACHABLE("missing");
         return -1;
   }
}

int
intel_perf_eustall_stream_open(struct intel_device_info *devinfo, int drm_fd,
                               uint32_t sample_rate, uint32_t min_event_count)
{
   if (devinfo->ver >= 20 &&
       devinfo->kmd_type == INTEL_KMD_TYPE_XE)
      return xe_perf_eustall_stream_open(drm_fd, sample_rate,
                                         min_event_count);
   return -1;
}

int
intel_perf_eustall_stream_set_state(struct intel_device_info *devinfo,
                                    int perf_stream_fd, bool enable)
{
   if (devinfo->ver >= 20 &&
       devinfo->kmd_type == INTEL_KMD_TYPE_XE)
      return xe_perf_stream_set_state(perf_stream_fd, enable);
   return -1;
}

int
intel_perf_eustall_stream_record_size(struct intel_device_info *devinfo,
                                      int drm_fd)
{
   if (devinfo->ver >= 20 &&
       devinfo->kmd_type == INTEL_KMD_TYPE_XE)
      return xe_perf_eustall_stream_record_size(drm_fd);
   return -1;
}

int
intel_perf_eustall_stream_sample_rate(struct intel_device_info *devinfo,
                                      int drm_fd)
{
   if (devinfo->ver >= 20 &&
       devinfo->kmd_type == INTEL_KMD_TYPE_XE)
      return xe_perf_eustall_stream_sample_rate(drm_fd);
   return -1;
}

int
intel_perf_eustall_stream_read_samples(struct intel_device_info *devinfo,
                                       int perf_stream_fd, uint8_t *buffer,
                                       size_t buffer_len, bool *overflow)
{
   if (devinfo->ver >= 20 &&
       devinfo->kmd_type == INTEL_KMD_TYPE_XE)
      return xe_perf_eustall_stream_read_samples(perf_stream_fd, buffer,
                                                 buffer_len, overflow);
   return -1;
}

void
intel_perf_eustall_accumulate_results(struct intel_perf_query_eustall_result *result,
                                      const void *start, const void *end,
                                      size_t record_size,
                                      int ver)
{
   return xe_perf_eustall_accumulate_results(result, start, end, record_size, ver);
}
