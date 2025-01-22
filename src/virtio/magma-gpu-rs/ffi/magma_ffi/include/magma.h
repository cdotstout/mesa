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

#ifdef __cplusplus
extern "C" {
#endif

#define MAGMA_MAX_SYNCOBJS 16
#define MAGMA_MAX_PHYSICAL_DEVICES 8
#define MAGMA_MAX_MEMORY_HEAPS 32
#define MAGMA_MAX_MEMORY_TYPES 16
#define MAGMA_MAX_QUEUES 16

struct magma_queue;
typedef struct magma_queue* magma_queue_t;

struct magma_address_space;
typedef struct magma_address_space* magma_address_space_t;

struct magma_buffer;
typedef struct magma_buffer* magma_buffer_t;

struct magma_sync_obj;
typedef struct magma_sync_obj* magma_sync_obj_t;

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
    MAGMA_STATUS_SUCCESS = 0,
    MAGMA_STATUS_INTERNAL_ERROR = -1,
    MAGMA_STATUS_INVALID_ARGS = -2,
    MAGMA_STATUS_ACCESS_DENIED = -3,
    MAGMA_STATUS_MEMORY_ERROR = -4,
    MAGMA_STATUS_CONTEXT_KILLED = -5,
    MAGMA_STATUS_TIMED_OUT = -6,
    MAGMA_STATUS_UNIMPLEMENTED = -7,
};

typedef uint32_t magma_sync_obj_type_t;
enum {
    MAGMA_SYNC_OBJ_TYPE_BINARY = 0,
    MAGMA_SYNC_OBJ_TYPE_TIMELINE = 1,
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

typedef uint32_t magma_sync_obj_use_flags_t;
enum {
    MAGMA_SYNC_OBJ_USE_FLAGS_EXPORTABLE = 0x00000001,
    MAGMA_SYNC_OBJ_USE_FLAGS_FENCE = 0x00000002,
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

struct magma_memory_properties {
    uint32_t memory_type_count;
    uint32_t memory_heap_count;
    struct magma_memory_type* memory_types;
    struct magma_heap* memory_heaps;
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
    MAGMA_STRUCTURE_TYPE_CREATE_SYNC_OBJ_INFO = 0x00000007,
    MAGMA_STRUCTURE_TYPE_SYNC_PROPERTIES = 0x00000008,
    MAGMA_STRUCTURE_TYPE_PHYSICAL_DEVICE_INFO = 0x00010002,
    MAGMA_STRUCTURE_TYPE_PHYSICAL_DEVICE_MEMORY_TYPE = 0x00010003,
    MAGMA_STRUCTURE_TYPE_PHYSICAL_DEVICE_MEMORY_HEAP = 0x00010004,
} magma_structure_type_t;

struct magma_structure_type_header {
    magma_structure_type_t s_type;
    uint32_t size;
    const void* p_next;
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
    uint32_t num_wait_sync_objs;
    uint32_t num_signal_sync_objs;
    magma_sync_obj_t wait_sync_objs[MAGMA_MAX_SYNCOBJS];
    magma_sync_obj_t signal_sync_objs[MAGMA_MAX_SYNCOBJS];
    uint64_t wait_points[MAGMA_MAX_SYNCOBJS];
    uint64_t signal_points[MAGMA_MAX_SYNCOBJS];
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

struct magma_create_sync_obj_info {
    struct magma_structure_type_header header;
    uint32_t sync_type;
    magma_sync_obj_use_flags_t use_flags;
    uint64_t initial_point;
};

struct magma_sync_properties {
    struct magma_structure_type_header header;
    uint32_t capabilities;
    uint32_t _pad0;
    uint64_t max_timeline_value_difference;
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

struct magma_physical_device_memory_type {
    struct magma_structure_type_header header;
    uint32_t property_flags;
    uint32_t heap_idx;
};

struct magma_physical_device_memory_heap {
    struct magma_structure_type_header header;
    uint64_t heap_size;
    uint64_t heap_flags;
};

magma_status_t magma_enumerate_physical_devices(magma_physical_device_t physical_devices[MAGMA_MAX_PHYSICAL_DEVICES],
                                                uint32_t* num_devices);

magma_status_t magma_create_device(magma_physical_device_t physical_device, magma_device_t* device);

magma_status_t magma_get_memory_budget(magma_device_t device, uint32_t heap_idx,
                                       struct magma_heap_budget* budget);

magma_status_t magma_create_buffer(magma_device_t device,
                                   const struct magma_create_buffer_info* info,
                                   magma_buffer_t* buffer_out);

magma_status_t magma_create_address_space(magma_device_t device,
                                          magma_address_space_t* address_space);

magma_status_t magma_create_queue(magma_device_t device, magma_address_space_t address_space,
                                  const struct magma_create_queue_info* info, magma_queue_t* queue);

magma_status_t magma_device_close(magma_device_t* device);

magma_status_t magma_physical_device_close(magma_physical_device_t* physical_device);

magma_status_t magma_buffer_close(magma_buffer_t* buffer);

magma_status_t magma_queue_close(magma_queue_t* queue);

magma_status_t magma_address_space_close(magma_address_space_t* address_space);

magma_status_t magma_map_buffer_gpu(magma_address_space_t address_space, magma_buffer_t buffer,
                                    uint64_t buffer_offset, uint64_t gpu_va, uint64_t size,
                                    magma_gpu_map_flags_t flags);

magma_status_t magma_unmap_buffer_gpu(magma_address_space_t address_space, uint64_t gpu_va,
                                      uint64_t size);

magma_status_t magma_submit_command(magma_queue_t queue,
                                    const struct magma_submit_info* submit_info);

magma_status_t magma_create_sync_obj(magma_device_t device,
                                     const struct magma_create_sync_obj_info* info,
                                     magma_sync_obj_t* sync_obj);

magma_status_t magma_sync_obj_close(magma_sync_obj_t* sync_obj);

magma_status_t magma_sync_obj_wait(magma_sync_obj_t sync_obj, uint64_t timeout_ns);

magma_status_t magma_sync_obj_signal(magma_sync_obj_t sync_obj);

magma_status_t magma_sync_obj_timeline_wait(magma_sync_obj_t sync_obj, uint64_t point,
                                            uint64_t timeout_ns, uint32_t flags);

magma_status_t magma_sync_obj_timeline_signal(magma_sync_obj_t sync_obj, uint64_t point);

magma_status_t magma_sync_obj_timeline_query(magma_sync_obj_t sync_obj, uint64_t* point_out);

magma_status_t magma_get_sync_properties(magma_device_t device,
                                         struct magma_sync_properties* properties);

void magma_get_physical_device_info(magma_physical_device_t physical_device,
                                    struct magma_physical_device_info* info);

magma_status_t magma_get_memory_properties(magma_physical_device_t physical_device,
                                           struct magma_memory_properties* memory_properties);

magma_status_t magma_queue_check_status(magma_queue_t queue);

magma_status_t magma_map_buffer_cpu(magma_buffer_t buffer, struct magma_mapping* mapping_out);

magma_status_t magma_unmap_buffer_cpu(magma_buffer_t buffer);

magma_status_t magma_sync_obj_export(magma_sync_obj_t sync_obj, struct magma_handle* handle_out);

magma_status_t magma_sync_obj_import(magma_sync_obj_t sync_obj, const struct magma_handle* handle);

magma_status_t magma_buffer_export(magma_buffer_t buffer, struct magma_handle* handle_out);

magma_status_t magma_buffer_import(magma_device_t device, const struct magma_handle* handle,
                                   magma_buffer_t* buffer_out);

#ifdef __cplusplus
}
#endif

#endif
