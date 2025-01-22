// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT
//
// Generated via:
//   https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/magma/gorgonzola
//
// Submit patches, do not hand-edit.

#ifndef MAGMA_H
#define MAGMA_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdalign.h>

#ifdef __cplusplus
extern "C" {
#endif

#define MAGMA_MAX_PHYSICAL_DEVICES 8
#define MAGMA_MAX_MEMORY_HEAPS 32
#define MAGMA_MAX_MEMORY_TYPES 16
#define MAGMA_MAX_QUEUES 16
#define MAGMA_MAX_HW_CONFIG_DWORDS 256

struct magma_queue;
typedef struct magma_queue* magma_queue_t;

struct magma_address_space;
typedef struct magma_address_space* magma_address_space_t;

struct magma_buffer;
typedef struct magma_buffer* magma_buffer_t;

struct magma_semaphore;
typedef struct magma_semaphore* magma_semaphore_t;

struct magma_physical_device;
typedef struct magma_physical_device* magma_physical_device_t;

struct magma_device;
typedef struct magma_device* magma_device_t;

typedef uint16_t magma_vendor_id_t;
enum {
    MAGMA_VENDOR_ID_UNDEFINED = 0x0000,
    MAGMA_VENDOR_ID_AMD = 0x1002,
    MAGMA_VENDOR_ID_ARM = 0x13B5,
    MAGMA_VENDOR_ID_VIRT_GPU = 0x1AF4,
    MAGMA_VENDOR_ID_QUALCOMM = 0x5413,
    MAGMA_VENDOR_ID_INTEL = 0x8086,
};

typedef int32_t magma_status_t;
enum {
    MAGMA_STATUS_SUCCESS = 0x00000000,
    MAGMA_STATUS_INTERNAL_ERROR = -1,
    MAGMA_STATUS_INVALID_ARGS = -2,
    MAGMA_STATUS_ACCESS_DENIED = -3,
    MAGMA_STATUS_MEMORY_ERROR = -4,
    MAGMA_STATUS_CONTEXT_KILLED = -5,
    MAGMA_STATUS_TIMED_OUT = -6,
    MAGMA_STATUS_UNIMPLEMENTED = -7,
};

typedef uint32_t magma_semaphore_type_t;
enum {
    MAGMA_SEMAPHORE_TYPE_BINARY = 0x00000000,
    MAGMA_SEMAPHORE_TYPE_TIMELINE = 0x00000001,
};

typedef uint32_t magma_memory_property_t;
enum {
    MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT = 0x00000001,
    MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT = 0x00000002,
    MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT = 0x00000004,
    MAGMA_MEMORY_PROPERTY_HOST_CACHED_BIT = 0x00000008,
    MAGMA_MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT = 0x00000010,
    MAGMA_MEMORY_PROPERTY_PROTECTED_BIT = 0x00000020,
};

typedef uint32_t magma_queue_flags_t;
enum {
    MAGMA_QUEUE_FLAGS_GRAPHICS = 0x00000001,
    MAGMA_QUEUE_FLAGS_COMPUTE = 0x00000002,
    MAGMA_QUEUE_FLAGS_TRANSFER = 0x00000004,
    MAGMA_QUEUE_FLAGS_SPARSE_BINDING = 0x00000008,
    MAGMA_QUEUE_FLAGS_PROTECTED = 0x00000010,
};

typedef uint64_t magma_gpu_map_flags_t;
enum {
    MAGMA_GPU_MAP_FLAGS_READ = 0x0000000000000001,
    MAGMA_GPU_MAP_FLAGS_WRITE = 0x0000000000000002,
    MAGMA_GPU_MAP_FLAGS_EXECUTE = 0x0000000000000004,
    MAGMA_GPU_MAP_FLAGS_GROW_UP = 0x0000000000000008,
    MAGMA_GPU_MAP_FLAGS_GROW_DOWN = 0x0000000000000010,
};

typedef uint32_t magma_buffer_flag_t;
enum {
    MAGMA_BUFFER_FLAG_EXTERNAL = 0x00000001,
    MAGMA_BUFFER_FLAG_SCANOUT = 0x00000002,
};

typedef uint32_t magma_sync_capability_t;
enum {
    MAGMA_SYNC_CAPABILITY_BINARY = 0x00000001,
    MAGMA_SYNC_CAPABILITY_TIMELINE = 0x00000002,
    MAGMA_SYNC_CAPABILITY_CPU_WAIT = 0x00000004,
    MAGMA_SYNC_CAPABILITY_CPU_SIGNAL = 0x00000008,
    MAGMA_SYNC_CAPABILITY_EXPORT_SYNC_FILE = 0x00000010,
    MAGMA_SYNC_CAPABILITY_IMPORT_SYNC_FILE = 0x00000020,
};

typedef uint32_t magma_semaphore_use_flags_t;
enum {
    MAGMA_SEMAPHORE_USE_FLAGS_EXPORTABLE = 0x00000001,
    MAGMA_SEMAPHORE_USE_FLAGS_FENCE = 0x00000002,
};

typedef uint32_t magma_semaphore_wait_flags_t;
enum {
    MAGMA_SEMAPHORE_WAIT_FLAGS_WAIT_PENDING = 0x00000001,
};

typedef uint32_t magma_handle_type_t;
enum {
    MAGMA_HANDLE_TYPE_MEM_OPAQUE_FD = 0x00000001,
    MAGMA_HANDLE_TYPE_MEM_DMABUF = 0x00000002,
    MAGMA_HANDLE_TYPE_MEM_OPAQUE_WIN32 = 0x00000003,
    MAGMA_HANDLE_TYPE_MEM_SHM = 0x00000004,
    MAGMA_HANDLE_TYPE_MEM_ZIRCON = 0x00000005,
    MAGMA_HANDLE_TYPE_SIGNAL_OPAQUE_FD = 0x00000010,
    MAGMA_HANDLE_TYPE_SIGNAL_SYNC_FD = 0x00000020,
    MAGMA_HANDLE_TYPE_SIGNAL_OPAQUE_WIN32 = 0x00000030,
    MAGMA_HANDLE_TYPE_SIGNAL_ZIRCON = 0x00000040,
    MAGMA_HANDLE_TYPE_SIGNAL_EVENT_FD = 0x00000050,
};

typedef uint64_t magma_heap_flag_t;
enum {
    MAGMA_HEAP_FLAG_DEVICE_LOCAL_BIT = 0x0000000000000001,
    MAGMA_HEAP_FLAG_CPU_VISIBLE_BIT = 0x0000000000000010,
};

typedef uint32_t magma_intel_config_flag_t;
enum {
    MAGMA_INTEL_CONFIG_FLAG_HAS_VRAM = 0x00000001,
    MAGMA_INTEL_CONFIG_FLAG_HAS_LOW_LATENCY = 0x00000002,
    MAGMA_INTEL_CONFIG_FLAG_HAS_CPU_ADDR_MIRROR = 0x00000004,
    MAGMA_INTEL_CONFIG_FLAG_HAS_NO_COMPRESSION_HINT = 0x00000008,
    MAGMA_INTEL_CONFIG_FLAG_HAS_STATE_CACHE_PERF_FIX = 0x00000010,
    MAGMA_INTEL_CONFIG_FLAG_HAS_PURGING_SUPPORT = 0x00000020,
};

struct magma_heap {
    uint64_t heap_size;
    uint64_t heap_flags;
};

struct magma_heap_budget {
    uint64_t budget;
    uint64_t usage;
};

struct magma_memory_type {
    uint32_t property_flags;
    uint32_t heap_idx;
};

struct magma_pci_bus_info {
    uint16_t domain;
    uint16_t subvendor_id;
    uint16_t subdevice_id;
    uint8_t revision_id;
    uint8_t bus;
    uint8_t device;
    uint8_t function;
    uint8_t _padding[6];
};

struct magma_create_queue_info {
    uint32_t queue_family_idx;
    uint32_t priority;
    uint32_t flags;
    uint32_t _padding;
};

struct magma_queue_family_properties {
    magma_queue_flags_t queue_flags;
    uint32_t queue_count;
};

struct magma_mapping {
    void* ptr;
    uint64_t size;
};

struct magma_handle {
    int64_t os_handle;
    magma_handle_type_t handle_type;
};

typedef enum {
    MAGMA_STRUCTURE_TYPE_CREATE_BUFFER_INFO = 0x00000001,
    MAGMA_STRUCTURE_TYPE_DEVICE_CREATE_INFO = 0x00000002,
    MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO = 0x00000003,
    MAGMA_STRUCTURE_TYPE_SUBMIT_INFO = 0x00000004,
    MAGMA_STRUCTURE_TYPE_SUBMIT_ADDRESS_SPACE_INFO = 0x00000005,
    MAGMA_STRUCTURE_TYPE_SUBMIT_BUFFER_INFO = 0x00000006,
    MAGMA_STRUCTURE_TYPE_SUBMIT_MAP_BUFFER_INFO = 0x00000009,
    MAGMA_STRUCTURE_TYPE_SUBMIT_UNMAP_BUFFER_INFO = 0x0000000A,
    MAGMA_STRUCTURE_TYPE_SUBMIT_RESOURCES_INFO = 0x00000012,
    MAGMA_STRUCTURE_TYPE_CREATE_SEMAPHORE_INFO = 0x00000007,
    MAGMA_STRUCTURE_TYPE_SYNC_PROPERTIES = 0x00000008,
    MAGMA_STRUCTURE_TYPE_MEMORY_PROPERTIES = 0x0000000B,
    MAGMA_STRUCTURE_TYPE_PHYSICAL_DEVICE_INFO = 0x00010002,
    MAGMA_STRUCTURE_TYPE_ADDRESS_SPACE_PROPERTIES = 0x0000000C,
    MAGMA_STRUCTURE_TYPE_TIMESTAMP_PROPERTIES = 0x0000000D,
    MAGMA_STRUCTURE_TYPE_INTEL_DEVICE_INFO = 0x0000000E,
    MAGMA_STRUCTURE_TYPE_ADRENO_DEVICE_INFO = 0x0000000F,
    MAGMA_STRUCTURE_TYPE_ARM_DEVICE_INFO = 0x00000010,
    MAGMA_STRUCTURE_TYPE_AMDGPU_DEVICE_INFO = 0x00000011,
} magma_structure_type_t;

struct magma_structure_type_header {
    magma_structure_type_t s_type;
    uint32_t size;
    alignas(8) const void* p_next;
};

struct magma_create_buffer_info {
    struct magma_structure_type_header header;
    uint32_t memory_type_idx;
    uint32_t alignment;
    uint32_t common_flags;
    uint32_t vendor_flags;
    uint64_t size;
};

struct magma_device_create_info {
    struct magma_structure_type_header header;
    uint32_t flags;
    uint32_t _padding;
};

struct magma_submit_sync_info {
    struct magma_structure_type_header header;
    uint32_t num_wait_semaphores;
    uint32_t num_signal_semaphores;
    magma_semaphore_t* wait_semaphores;
    magma_semaphore_t* signal_semaphores;
    uint64_t* wait_points;
    uint64_t* signal_points;
};

struct magma_submit_info {
    struct magma_structure_type_header header;
    uint32_t flags;
    struct magma_submit_sync_info sync_info;
};

struct magma_submit_address_space_info {
    struct magma_structure_type_header header;
    uint64_t command_va;
    uint64_t length;
};

struct magma_submit_buffer_info {
    struct magma_structure_type_header header;
    magma_buffer_t command_buffer;
    uint64_t start_offset;
    uint64_t length;
};

struct magma_submit_map_buffer_info {
    struct magma_structure_type_header header;
    magma_buffer_t buffer;
    uint64_t buffer_offset;
    uint64_t gpu_va;
    uint64_t size;
    magma_gpu_map_flags_t flags;
};

struct magma_submit_unmap_buffer_info {
    struct magma_structure_type_header header;
    uint64_t gpu_va;
    uint64_t size;
};

struct magma_submit_resources_info {
    struct magma_structure_type_header header;
    uint32_t num_resources;
    magma_buffer_t* buffers;
};

struct magma_create_semaphore_info {
    struct magma_structure_type_header header;
    uint32_t semaphore_type;
    magma_semaphore_use_flags_t use_flags;
    uint64_t initial_point;
};

struct magma_sync_properties {
    struct magma_structure_type_header header;
    uint32_t capabilities;
    uint32_t _pad0;
    uint64_t max_timeline_value_difference;
};

struct magma_memory_properties {
    struct magma_structure_type_header header;
    uint32_t memory_type_count;
    uint32_t memory_heap_count;
    struct magma_memory_type memory_types[MAGMA_MAX_MEMORY_TYPES];
    struct magma_heap memory_heaps[MAGMA_MAX_MEMORY_HEAPS];
};

struct magma_physical_device_info {
    struct magma_structure_type_header header;
    uint32_t bus_type;
    magma_vendor_id_t vendor_id;
    uint16_t device_id;
    uint32_t flags;
    struct magma_pci_bus_info pci_bus_info;
    uint32_t queue_family_count;
    struct magma_queue_family_properties queue_families[MAGMA_MAX_QUEUES];
};

struct magma_address_space_properties {
    struct magma_structure_type_header header;
    uint64_t va_start;
    uint64_t va_size;
    uint64_t mem_alignment;
};

struct magma_timestamp_properties {
    struct magma_structure_type_header header;
    uint64_t timestamp_frequency;
};

struct magma_intel_device_info {
    struct magma_structure_type_header header;
    uint32_t hw_config_count;
    uint32_t l3_banks;
    uint64_t geo_dss_mask;
    uint64_t eu_per_dss_mask;
    uint32_t geo_dss_num_bytes;
    uint32_t ip_ver_major;
    uint32_t ip_ver_minor;
    uint32_t ip_ver_rev;
    magma_intel_config_flag_t config_flags;
    uint32_t hw_config_data[MAGMA_MAX_HW_CONFIG_DWORDS];
    uint32_t _padding;
};

struct magma_adreno_device_info {
    struct magma_structure_type_header header;
    uint64_t chip_id;
    uint64_t gmem_size;
    uint64_t gmem_base;
    uint32_t gpu_id;
    uint32_t highest_bank_bit;
    uint32_t ubwc_swizzle;
    uint32_t macrotile_mode;
    uint32_t raytracing;
    uint32_t flags;
};

struct magma_arm_device_info {
    struct magma_structure_type_header header;
    uint64_t shader_present;
    uint64_t tiler_present;
    uint64_t l2_present;
    uint32_t gpu_id;
    uint32_t gpu_rev;
    uint32_t csf_id;
    uint32_t l2_features;
    uint32_t tiler_features;
    uint32_t mem_features;
    uint32_t mmu_features;
    uint32_t thread_features;
    uint32_t max_threads;
    uint32_t thread_max_workgroup_size;
    uint32_t thread_max_barrier_size;
    uint32_t coherency_features;
    uint32_t selected_coherency;
    uint32_t as_present;
    uint32_t csg_slot_count;
    uint32_t cs_slot_count;
    uint32_t cs_reg_count;
    uint32_t scoreboard_slot_count;
};

struct magma_amdgpu_device_info {
    struct magma_structure_type_header header;
    uint64_t max_engine_clock;
    uint64_t max_memory_clock;
    uint64_t ids_flags;
    uint32_t chip_rev;
    uint32_t external_rev;
    uint32_t family;
    uint32_t num_shader_engines;
    uint32_t num_shader_arrays_per_engine;
    uint32_t cu_active_number;
    uint32_t enabled_rb_pipes_mask;
    uint32_t num_rb_pipes;
    uint32_t vram_type;
    uint32_t vram_bit_width;
    uint32_t wave_front_size;
    uint32_t num_shader_visible_vgprs;
    uint32_t num_cu_per_sh;
    uint32_t num_tcc_blocks;
};

magma_status_t magma_enumerate_physical_devices(magma_physical_device_t physical_devices[MAGMA_MAX_PHYSICAL_DEVICES],
                                                uint32_t* num_devices);

magma_status_t magma_physical_device_create_device(magma_physical_device_t physical_device,
                                                   magma_device_t* device);

magma_status_t magma_device_get_memory_budget(magma_device_t device, uint32_t heap_idx,
                                              struct magma_heap_budget* budget);

magma_status_t magma_device_create_buffer(magma_device_t device,
                                          const struct magma_create_buffer_info* info,
                                          magma_buffer_t* buffer_out);

magma_status_t magma_device_create_address_space(magma_device_t device,
                                                 magma_address_space_t* address_space);

magma_status_t magma_device_create_queue(magma_device_t device,
                                         magma_address_space_t address_space,
                                         const struct magma_create_queue_info* info,
                                         magma_queue_t* queue);

magma_status_t magma_device_release(magma_device_t* device);

magma_status_t magma_physical_device_release(magma_physical_device_t* physical_device);

magma_status_t magma_buffer_release(magma_buffer_t* buffer);

magma_status_t magma_queue_release(magma_queue_t* queue);

magma_status_t magma_address_space_release(magma_address_space_t* address_space);

magma_status_t magma_queue_submit(magma_queue_t queue, const struct magma_submit_info* submit_info);

magma_status_t magma_device_create_semaphore(magma_device_t device,
                                             const struct magma_create_semaphore_info* info,
                                             magma_semaphore_t* semaphore);

magma_status_t magma_semaphore_release(magma_semaphore_t* semaphore);

magma_status_t magma_semaphore_wait(magma_semaphore_t semaphore, uint64_t timeout_ns);

magma_status_t magma_semaphore_signal(magma_semaphore_t semaphore);

magma_status_t magma_semaphore_timeline_wait(magma_semaphore_t semaphore, uint64_t point,
                                             uint64_t timeout_ns, uint32_t flags);

magma_status_t magma_semaphore_timeline_signal(magma_semaphore_t semaphore, uint64_t point);

magma_status_t magma_semaphore_timeline_query(magma_semaphore_t semaphore, uint64_t* point_out);

magma_status_t magma_physical_device_query(magma_physical_device_t physical_device, void* query);

magma_status_t magma_queue_check_status(magma_queue_t queue);

magma_status_t magma_buffer_map(magma_buffer_t buffer, struct magma_mapping* mapping_out);

magma_status_t magma_buffer_unmap(magma_buffer_t buffer);

magma_status_t magma_semaphore_export(magma_semaphore_t semaphore, struct magma_handle* handle_out);

magma_status_t magma_semaphore_import(magma_semaphore_t semaphore,
                                      const struct magma_handle* handle);

magma_status_t magma_buffer_export(magma_buffer_t buffer, struct magma_handle* handle_out);

magma_status_t magma_device_import_buffer(magma_device_t device, const struct magma_handle* handle,
                                          magma_buffer_t* buffer_out);

#ifdef __cplusplus
}
#endif

#endif
