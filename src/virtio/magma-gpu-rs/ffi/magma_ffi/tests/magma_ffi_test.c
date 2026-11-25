// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

#include <assert.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "magma.h"
#include "magma_amdgpu.h"
#include "magma_intel.h"

struct vendor_noop_entry {
    magma_vendor_id_t vendor_id;
    uint32_t noop_cmd;
};

static const struct vendor_noop_entry vendor_noop_table[] = {
    { MAGMA_VENDOR_ID_AMD,   0xffff1000 }, /* PKT3_NOP */
    { MAGMA_VENDOR_ID_INTEL, 0x05000000 }, /* MI_BATCH_BUFFER_END */
};

static uint32_t get_vendor_noop_cmd(magma_vendor_id_t vendor_id) {
    for (size_t i = 0; i < sizeof(vendor_noop_table) / sizeof(vendor_noop_table[0]); i++) {
        if (vendor_noop_table[i].vendor_id == vendor_id) {
            return vendor_noop_table[i].noop_cmd;
        }
    }
    return 0;
}

struct test_runner {
    uint32_t num_physical_devices;
    magma_physical_device_t physical_devices[MAGMA_MAX_PHYSICAL_DEVICES];
    struct magma_physical_device_info dev_info;

    magma_device_t device;
    magma_address_space_t address_space;
    magma_queue_t queue;

    magma_buffer_t buffer;
    uint64_t buffer_size;
    uint64_t gpu_va;

    magma_semaphore_t semaphore;
    magma_semaphore_t out_semaphore;
};

static void test_enumerate_and_init(struct test_runner *runner) {
    magma_status_t status =
        magma_enumerate_physical_devices(runner->physical_devices, &runner->num_physical_devices);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->num_physical_devices > 0);
    assert(runner->physical_devices[0] != NULL);
    printf("Successfully enumerated %u physical device(s)\n", runner->num_physical_devices);

    struct magma_timestamp_properties ts_props = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_TIMESTAMP_PROPERTIES,
            .size = sizeof(ts_props),
            .p_next = NULL,
        },
    };
    struct magma_address_space_properties as_props = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_ADDRESS_SPACE_PROPERTIES,
            .size = sizeof(as_props),
            .p_next = &ts_props,
        },
    };
    struct magma_sync_properties sync_props = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_SYNC_PROPERTIES,
            .size = sizeof(sync_props),
            .p_next = &as_props,
        },
    };
    memset(&runner->dev_info, 0, sizeof(runner->dev_info));
    runner->dev_info.header.s_type = MAGMA_STRUCTURE_TYPE_PHYSICAL_DEVICE_INFO;
    runner->dev_info.header.size = sizeof(runner->dev_info);
    runner->dev_info.header.p_next = &sync_props;

    status = magma_physical_device_query(runner->physical_devices[0], &runner->dev_info);
    assert(status == MAGMA_STATUS_SUCCESS);

    printf("Physical Device Info:\n");
    printf("  Vendor ID: 0x%04x, Device ID: 0x%04x\n",
           runner->dev_info.vendor_id, runner->dev_info.device_id);

    static const magma_vendor_id_t known_vendor_ids[] = {
        MAGMA_VENDOR_ID_AMD,
        MAGMA_VENDOR_ID_ARM,
        MAGMA_VENDOR_ID_INTEL,
        MAGMA_VENDOR_ID_QUALCOMM,
        MAGMA_VENDOR_ID_VIRT_GPU,
    };
    bool valid_vendor = false;
    for (size_t i = 0; i < sizeof(known_vendor_ids) / sizeof(known_vendor_ids[0]); i++) {
        if (runner->dev_info.vendor_id == known_vendor_ids[i]) {
            valid_vendor = true;
            break;
        }
    }
    assert(valid_vendor);
    printf("  Queue family count: %u\n", runner->dev_info.queue_family_count);
    for (uint32_t i = 0; i < runner->dev_info.queue_family_count; i++) {
        printf("    Queue family %u: count %u, flags 0x%08x\n",
               i,
               runner->dev_info.queue_families[i].queue_count,
               runner->dev_info.queue_families[i].queue_flags);
    }

    assert(sync_props.capabilities != 0);
    printf("  Sync capabilities: 0x%08x, max_timeline_diff: 0x%lx\n",
           sync_props.capabilities,
           (unsigned long)sync_props.max_timeline_value_difference);
    printf("  Address space: va_start 0x%lx, va_size 0x%lx, mem_alignment %lu\n",
           (unsigned long)as_props.va_start,
           (unsigned long)as_props.va_size,
           (unsigned long)as_props.mem_alignment);
    printf("  Timestamp frequency: %lu Hz\n",
           (unsigned long)ts_props.timestamp_frequency);

    if (runner->dev_info.vendor_id == MAGMA_VENDOR_ID_AMD) {
        struct magma_amdgpu_device_info amd_info = { 0 };
        struct magma_vendor_query amd_query = {
            .header = {
                .s_type = MAGMA_STRUCTURE_TYPE_VENDOR_QUERY,
                .size = sizeof(amd_query),
                .p_next = NULL,
            },
            .query_id = MAGMA_AMDGPU_QUERY_DEVICE_INFO,
            .buffer = &amd_info,
            .buffer_size = sizeof(amd_info),
        };
        status = magma_physical_device_query(runner->physical_devices[0], &amd_query);
        assert(status == MAGMA_STATUS_SUCCESS);
        printf("  AMDGPU Device Info: family %u, chip_rev %u, cu_active %u\n",
               amd_info.family, amd_info.chip_rev, amd_info.cu_active_number);
    } else if (runner->dev_info.vendor_id == MAGMA_VENDOR_ID_INTEL) {
        struct magma_intel_device_info intel_info = { 0 };
        struct magma_vendor_query intel_query = {
            .header = {
                .s_type = MAGMA_STRUCTURE_TYPE_VENDOR_QUERY,
                .size = sizeof(intel_query),
                .p_next = NULL,
            },
            .query_id = MAGMA_INTEL_QUERY_DEVICE_INFO,
            .buffer = &intel_info,
            .buffer_size = sizeof(intel_info),
        };
        status = magma_physical_device_query(runner->physical_devices[0], &intel_query);
        assert(status == MAGMA_STATUS_SUCCESS);
        printf("  Intel Device Info: ip_ver %u.%u.%u, geo_dss_mask 0x%lx\n",
               intel_info.ip_ver_major, intel_info.ip_ver_minor,
               intel_info.ip_ver_rev, (unsigned long)intel_info.geo_dss_mask);
    }

    status = magma_physical_device_create_device(runner->physical_devices[0], &runner->device);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->device != NULL);
    printf("Successfully created device\n");
}

static void test_create_resources(struct test_runner *runner) {
    magma_status_t status = magma_device_create_address_space(runner->device, &runner->address_space);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->address_space != NULL);

    struct magma_create_queue_info queue_info = {
        .queue_family_idx = 0,
        .priority = 0,
    };
    status = magma_device_create_queue(runner->device, runner->address_space, &queue_info, &runner->queue);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->queue != NULL);

    runner->buffer_size = 4096;
    runner->gpu_va = 0x20000000;
    struct magma_create_buffer_info create_buffer_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_BUFFER_INFO,
            .size = sizeof(create_buffer_info),
        },
        .alignment = 4096,
        .common_flags = MAGMA_BUFFER_FLAG_CACHED,
        .vendor_flags = 0,
        .size = runner->buffer_size,
    };
    status = magma_device_create_buffer(runner->device, &create_buffer_info, &runner->buffer);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->buffer != NULL);
}

static void test_cpu_buffer_mapping(struct test_runner *runner) {
    struct magma_mapping mapping;
    memset(&mapping, 0, sizeof(mapping));
    magma_status_t status = magma_buffer_map(runner->buffer, &mapping);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(mapping.ptr != NULL);
    assert(mapping.size >= runner->buffer_size);

    uint32_t noop_cmd = get_vendor_noop_cmd(runner->dev_info.vendor_id);
    uint32_t *words = (uint32_t*)mapping.ptr;
    for (size_t i = 0; i < mapping.size / 4; i++) {
        words[i] = noop_cmd;
    }

    status = magma_buffer_unmap(runner->buffer);
    assert(status == MAGMA_STATUS_SUCCESS);
}

static magma_status_t submit_map_buffer(magma_queue_t queue,
                                        magma_buffer_t buffer,
                                        uint64_t buffer_offset,
                                        uint64_t gpu_va,
                                        uint64_t size,
                                        magma_gpu_map_flags_t flags) {
    struct magma_submit_map_buffer_info map_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_MAP_BUFFER_INFO,
            .size = sizeof(map_info),
            .p_next = NULL,
        },
        .buffer = buffer,
        .buffer_offset = buffer_offset,
        .gpu_va = gpu_va,
        .size = size,
        .flags = flags,
    };
    struct magma_submit_info submit_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_INFO,
            .size = sizeof(submit_info),
            .p_next = &map_info,
        },
        .flags = 0,
        .sync_info = {
            .header = {
                .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO,
                .size = sizeof(struct magma_submit_sync_info),
                .p_next = NULL,
            },
        },
    };
    return magma_queue_submit(queue, &submit_info);
}

static magma_status_t submit_unmap_buffer(magma_queue_t queue,
                                          uint64_t gpu_va,
                                          uint64_t size) {
    struct magma_submit_unmap_buffer_info unmap_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_UNMAP_BUFFER_INFO,
            .size = sizeof(unmap_info),
            .p_next = NULL,
        },
        .gpu_va = gpu_va,
        .size = size,
    };
    struct magma_submit_info submit_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_INFO,
            .size = sizeof(submit_info),
            .p_next = &unmap_info,
        },
        .flags = 0,
        .sync_info = {
            .header = {
                .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO,
                .size = sizeof(struct magma_submit_sync_info),
                .p_next = NULL,
            },
        },
    };
    return magma_queue_submit(queue, &submit_info);
}

static void test_gpu_buffer_mapping(struct test_runner *runner) {
    magma_status_t status = submit_map_buffer(
        runner->queue, runner->buffer, 0, runner->gpu_va, runner->buffer_size,
        MAGMA_GPU_MAP_FLAGS_READ | MAGMA_GPU_MAP_FLAGS_WRITE | MAGMA_GPU_MAP_FLAGS_EXECUTE);
    assert(status == MAGMA_STATUS_SUCCESS);
}

static void test_semaphore(struct test_runner *runner) {
    struct magma_create_semaphore_info sem_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SEMAPHORE_INFO,
            .size = sizeof(sem_info),
        },
        .semaphore_type = MAGMA_SEMAPHORE_TYPE_BINARY,
        .use_flags = MAGMA_SEMAPHORE_USE_FLAGS_FENCE,
        .initial_point = 0,
    };
    magma_status_t status = magma_device_create_semaphore(runner->device, &sem_info, &runner->semaphore);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->semaphore != NULL);

    status = magma_semaphore_signal(runner->semaphore);
    assert(status == MAGMA_STATUS_SUCCESS);

    status = magma_semaphore_wait(runner->semaphore, 1000000000ULL);
    assert(status == MAGMA_STATUS_SUCCESS);
}

static void test_submit_command(struct test_runner *runner) {
    struct magma_create_semaphore_info sem_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SEMAPHORE_INFO,
            .size = sizeof(sem_info),
        },
        .semaphore_type = MAGMA_SEMAPHORE_TYPE_BINARY,
        .use_flags = MAGMA_SEMAPHORE_USE_FLAGS_EXPORTABLE,
        .initial_point = 0,
    };
    magma_status_t status = magma_device_create_semaphore(runner->device, &sem_info, &runner->out_semaphore);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->out_semaphore != NULL);

    struct magma_submit_address_space_info addr_info;
    memset(&addr_info, 0, sizeof(addr_info));
    addr_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_ADDRESS_SPACE_INFO;
    addr_info.header.size = sizeof(addr_info);
    addr_info.command_va = runner->gpu_va;
    addr_info.length = 256;

    struct magma_submit_info submit_info;
    memset(&submit_info, 0, sizeof(submit_info));
    submit_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_INFO;
    submit_info.header.size = sizeof(submit_info);
    submit_info.header.p_next = &addr_info;
    submit_info.flags = 0;

    submit_info.sync_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO;
    submit_info.sync_info.header.size = sizeof(submit_info.sync_info);
    submit_info.sync_info.num_wait_semaphores = 1;
    submit_info.sync_info.wait_semaphores = &runner->semaphore;
    submit_info.sync_info.num_signal_semaphores = 1;
    submit_info.sync_info.signal_semaphores = &runner->out_semaphore;

    status = magma_queue_submit(runner->queue, &submit_info);
    assert(status == MAGMA_STATUS_SUCCESS);
}

static void test_export_fence(struct test_runner *runner) {
    struct magma_handle handle;
    memset(&handle, 0, sizeof(handle));
    magma_status_t status = magma_semaphore_export(runner->out_semaphore, &handle);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(handle.os_handle >= 0);
    assert(handle.handle_type != 0);
    close((int)handle.os_handle);
}

static void test_export_buffer(struct test_runner *runner) {
    struct magma_handle handle;
    memset(&handle, 0, sizeof(handle));
    magma_status_t status = magma_buffer_export(runner->buffer, &handle);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(handle.os_handle >= 0);
    printf("Successfully exported buffer: fd=%d type=%u\n", (int)handle.os_handle, handle.handle_type);

    magma_buffer_t imported_buf = NULL;
    status = magma_device_import_buffer(runner->device, &handle, &imported_buf);
    if (status == MAGMA_STATUS_SUCCESS) {
        assert(imported_buf != NULL);
        status = magma_buffer_release(&imported_buf);
        assert(status == MAGMA_STATUS_SUCCESS);
        assert(imported_buf == NULL);
        printf("Successfully re-imported and released buffer without closing original GEM handle!\n");
    } else {
        assert(status == MAGMA_STATUS_UNIMPLEMENTED);
    }
}

static void test_import_semaphore(struct test_runner *runner) {
    struct magma_handle handle;
    memset(&handle, 0, sizeof(handle));
    magma_status_t status = magma_semaphore_export(runner->out_semaphore, &handle);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(handle.os_handle >= 0);

    struct magma_create_semaphore_info sem_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SEMAPHORE_INFO,
            .size = sizeof(sem_info),
        },
        .semaphore_type = MAGMA_SEMAPHORE_TYPE_BINARY,
        .use_flags = MAGMA_SEMAPHORE_USE_FLAGS_FENCE,
        .initial_point = 0,
    };
    magma_semaphore_t imported_semaphore = NULL;
    status = magma_device_create_semaphore(runner->device, &sem_info, &imported_semaphore);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(imported_semaphore != NULL);

    status = magma_semaphore_import(imported_semaphore, &handle);
    assert(status == MAGMA_STATUS_SUCCESS);
    printf("Successfully imported semaphore: fd=%d\n", (int)handle.os_handle);

    // Submit a command waiting on the imported semaphore to verify guest-side fence wait.
    struct magma_submit_address_space_info addr_info;
    memset(&addr_info, 0, sizeof(addr_info));
    addr_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_ADDRESS_SPACE_INFO;
    addr_info.header.size = sizeof(addr_info);
    addr_info.command_va = runner->gpu_va;
    addr_info.length = 256;

    struct magma_submit_info submit_info;
    memset(&submit_info, 0, sizeof(submit_info));
    submit_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_INFO;
    submit_info.header.size = sizeof(submit_info);
    submit_info.header.p_next = &addr_info;
    submit_info.flags = 0;

    submit_info.sync_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO;
    submit_info.sync_info.header.size = sizeof(submit_info.sync_info);
    submit_info.sync_info.num_wait_semaphores = 1;
    submit_info.sync_info.wait_semaphores = &imported_semaphore;
    submit_info.sync_info.num_signal_semaphores = 0;

    status = magma_queue_submit(runner->queue, &submit_info);
    assert(status == MAGMA_STATUS_SUCCESS);
    printf("Successfully submitted command waiting on imported semaphore!\n");

    status = magma_semaphore_release(&imported_semaphore);
    assert(status == MAGMA_STATUS_SUCCESS);
}

static void test_multiple_submits(struct test_runner *runner) {
    printf("Testing multiple submits with fresh semaphores...\n");
    for (int i = 0; i < 20; i++) {
        struct magma_create_semaphore_info sem_info = {
            .header = {
                .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SEMAPHORE_INFO,
                .size = sizeof(sem_info),
            },
            .semaphore_type = MAGMA_SEMAPHORE_TYPE_BINARY,
            .use_flags = MAGMA_SEMAPHORE_USE_FLAGS_FENCE | MAGMA_SEMAPHORE_USE_FLAGS_EXPORTABLE,
            .initial_point = 0,
        };
        magma_semaphore_t wait_sem = NULL;
        magma_semaphore_t sig_sem = NULL;
        magma_status_t status = magma_device_create_semaphore(runner->device, &sem_info, &wait_sem);
        assert(status == MAGMA_STATUS_SUCCESS);
        status = magma_semaphore_signal(wait_sem);
        assert(status == MAGMA_STATUS_SUCCESS);
        status = magma_device_create_semaphore(runner->device, &sem_info, &sig_sem);
        assert(status == MAGMA_STATUS_SUCCESS);

        struct magma_submit_address_space_info addr_info;
        memset(&addr_info, 0, sizeof(addr_info));
        addr_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_ADDRESS_SPACE_INFO;
        addr_info.header.size = sizeof(addr_info);
        addr_info.command_va = runner->gpu_va;
        addr_info.length = 256;

        struct magma_submit_info submit_info;
        memset(&submit_info, 0, sizeof(submit_info));
        submit_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_INFO;
        submit_info.header.size = sizeof(submit_info);
        submit_info.header.p_next = &addr_info;
        submit_info.flags = 0;

        submit_info.sync_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO;
        submit_info.sync_info.header.size = sizeof(submit_info.sync_info);
        submit_info.sync_info.num_wait_semaphores = 1;
        submit_info.sync_info.wait_semaphores = &wait_sem;
        submit_info.sync_info.num_signal_semaphores = 1;
        submit_info.sync_info.signal_semaphores = &sig_sem;

        status = magma_queue_submit(runner->queue, &submit_info);
        assert(status == MAGMA_STATUS_SUCCESS);

        status = magma_semaphore_release(&wait_sem);
        assert(status == MAGMA_STATUS_SUCCESS);
        status = magma_semaphore_release(&sig_sem);
        assert(status == MAGMA_STATUS_SUCCESS);
    }
    printf("Completed 20 submits successfully!\n");
}

static void test_timeline_semaphore(struct test_runner *runner) {
    printf("Testing timeline semaphores (CPU signal + GPU wait/signal + query)...\n");
    struct magma_create_semaphore_info sem_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SEMAPHORE_INFO,
            .size = sizeof(sem_info),
        },
        .semaphore_type = MAGMA_SEMAPHORE_TYPE_TIMELINE,
        .use_flags = MAGMA_SEMAPHORE_USE_FLAGS_FENCE,
        .initial_point = 0,
    };
    magma_semaphore_t timeline_in = NULL;
    magma_semaphore_t timeline_out = NULL;
    magma_status_t status = magma_device_create_semaphore(runner->device, &sem_info, &timeline_in);
    assert(status == MAGMA_STATUS_SUCCESS);
    status = magma_device_create_semaphore(runner->device, &sem_info, &timeline_out);
    assert(status == MAGMA_STATUS_SUCCESS);

    // 1. CPU timeline signal, query, and wait
    status = magma_semaphore_timeline_signal(timeline_in, 5);
    assert(status == MAGMA_STATUS_SUCCESS);

    uint64_t val = 0;
    status = magma_semaphore_timeline_query(timeline_in, &val);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(val == 5);

    status = magma_semaphore_timeline_wait(timeline_in, 5, UINT64_MAX, 0);
    assert(status == MAGMA_STATUS_SUCCESS);

    // 2. GPU submit waiting on timeline_in(5) and signaling timeline_out(42)
    struct magma_submit_address_space_info addr_info;
    memset(&addr_info, 0, sizeof(addr_info));
    addr_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_ADDRESS_SPACE_INFO;
    addr_info.header.size = sizeof(addr_info);
    addr_info.command_va = runner->gpu_va;
    addr_info.length = 256;

    struct magma_submit_info submit_info;
    memset(&submit_info, 0, sizeof(submit_info));
    submit_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_INFO;
    submit_info.header.size = sizeof(submit_info);
    submit_info.header.p_next = &addr_info;
    submit_info.flags = 0;

    uint64_t wait_point = 5;
    uint64_t signal_point = 42;
    submit_info.sync_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO;
    submit_info.sync_info.header.size = sizeof(submit_info.sync_info);
    submit_info.sync_info.num_wait_semaphores = 1;
    submit_info.sync_info.wait_semaphores = &timeline_in;
    submit_info.sync_info.wait_points = &wait_point;
    submit_info.sync_info.num_signal_semaphores = 1;
    submit_info.sync_info.signal_semaphores = &timeline_out;
    submit_info.sync_info.signal_points = &signal_point;

    status = magma_queue_submit(runner->queue, &submit_info);
    assert(status == MAGMA_STATUS_SUCCESS);

    status = magma_semaphore_timeline_wait(timeline_out, 42, UINT64_MAX, 0);
    assert(status == MAGMA_STATUS_SUCCESS);

    val = 0;
    status = magma_semaphore_timeline_query(timeline_out, &val);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(val >= 42);

    status = magma_semaphore_release(&timeline_in);
    assert(status == MAGMA_STATUS_SUCCESS);
    status = magma_semaphore_release(&timeline_out);
    assert(status == MAGMA_STATUS_SUCCESS);
    printf("Timeline semaphore test passed!\n");
}

static void test_many_semaphores_submit(struct test_runner *runner) {
    enum { NUM_MANY_SEMS = 24 };
    printf("Testing single submit with %d wait and %d signal semaphores (dynamic array)...\n",
           NUM_MANY_SEMS, NUM_MANY_SEMS);
    struct magma_create_semaphore_info sem_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SEMAPHORE_INFO,
            .size = sizeof(sem_info),
        },
        .semaphore_type = MAGMA_SEMAPHORE_TYPE_TIMELINE,
        .use_flags = MAGMA_SEMAPHORE_USE_FLAGS_FENCE,
        .initial_point = 0,
    };
    magma_semaphore_t wait_sems[NUM_MANY_SEMS];
    magma_semaphore_t signal_sems[NUM_MANY_SEMS];
    uint64_t wait_pts[NUM_MANY_SEMS];
    uint64_t signal_pts[NUM_MANY_SEMS];
    magma_status_t status;

    for (int i = 0; i < NUM_MANY_SEMS; i++) {
        wait_pts[i] = (uint64_t)(i + 1);
        signal_pts[i] = (uint64_t)(100 + i);
        status = magma_device_create_semaphore(runner->device, &sem_info, &wait_sems[i]);
        assert(status == MAGMA_STATUS_SUCCESS);
        status = magma_semaphore_timeline_signal(wait_sems[i], wait_pts[i]);
        assert(status == MAGMA_STATUS_SUCCESS);
        status = magma_device_create_semaphore(runner->device, &sem_info, &signal_sems[i]);
        assert(status == MAGMA_STATUS_SUCCESS);
    }

    struct magma_submit_address_space_info addr_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_ADDRESS_SPACE_INFO,
            .size = sizeof(addr_info),
            .p_next = NULL,
        },
        .command_va = runner->gpu_va,
        .length = 256,
    };

    struct magma_submit_info submit_info;
    memset(&submit_info, 0, sizeof(submit_info));
    submit_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_INFO;
    submit_info.header.size = sizeof(submit_info);
    submit_info.header.p_next = &addr_info;
    submit_info.sync_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO;
    submit_info.sync_info.header.size = sizeof(submit_info.sync_info);
    submit_info.sync_info.num_wait_semaphores = NUM_MANY_SEMS;
    submit_info.sync_info.wait_semaphores = wait_sems;
    submit_info.sync_info.wait_points = wait_pts;
    submit_info.sync_info.num_signal_semaphores = NUM_MANY_SEMS;
    submit_info.sync_info.signal_semaphores = signal_sems;
    submit_info.sync_info.signal_points = signal_pts;

    status = magma_queue_submit(runner->queue, &submit_info);
    assert(status == MAGMA_STATUS_SUCCESS);

    for (int i = 0; i < NUM_MANY_SEMS; i++) {
        status = magma_semaphore_timeline_wait(signal_sems[i], signal_pts[i], UINT64_MAX, 0);
        assert(status == MAGMA_STATUS_SUCCESS);
        status = magma_semaphore_release(&wait_sems[i]);
        assert(status == MAGMA_STATUS_SUCCESS);
        status = magma_semaphore_release(&signal_sems[i]);
        assert(status == MAGMA_STATUS_SUCCESS);
    }
    printf("Dynamic array submit with %d wait and %d signal semaphores passed!\n",
           NUM_MANY_SEMS, NUM_MANY_SEMS);
}

struct async_submit_args {
    struct test_runner *runner;
    magma_semaphore_t binary_sem;
    magma_semaphore_t timeline_sem;
};

static void *async_submit_thread_fn(void *arg) {
    struct async_submit_args *args = (struct async_submit_args *)arg;
    // Sleep 25ms so the main thread enters wait() / export() BEFORE submit_command runs.
    usleep(25000);

    struct magma_submit_address_space_info addr_info;
    memset(&addr_info, 0, sizeof(addr_info));
    addr_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_ADDRESS_SPACE_INFO;
    addr_info.header.size = sizeof(addr_info);
    addr_info.command_va = args->runner->gpu_va;
    addr_info.length = 256;

    struct magma_submit_info submit_info;
    memset(&submit_info, 0, sizeof(submit_info));
    submit_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_INFO;
    submit_info.header.size = sizeof(submit_info);
    submit_info.header.p_next = &addr_info;
    submit_info.flags = 0;

    magma_semaphore_t signal_sems[2] = { args->binary_sem, args->timeline_sem };
    uint64_t signal_pts[2] = { 0, 100 };
    submit_info.sync_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO;
    submit_info.sync_info.header.size = sizeof(submit_info.sync_info);
    submit_info.sync_info.num_wait_semaphores = 0;
    submit_info.sync_info.num_signal_semaphores = 2;
    submit_info.sync_info.signal_semaphores = signal_sems;
    submit_info.sync_info.signal_points = signal_pts;

    magma_status_t status = magma_queue_submit(args->runner->queue, &submit_info);
    assert(status == MAGMA_STATUS_SUCCESS);
    return NULL;
}

static void test_threaded_wait_for_submit(struct test_runner *runner) {
    printf("Testing threaded WAIT_FOR_SUBMIT + export_fence (vk_queue_submit_thread pattern)...\n");
    struct magma_create_semaphore_info bin_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SEMAPHORE_INFO,
            .size = sizeof(bin_info),
        },
        .semaphore_type = MAGMA_SEMAPHORE_TYPE_BINARY,
        .use_flags = MAGMA_SEMAPHORE_USE_FLAGS_EXPORTABLE,
        .initial_point = 0,
    };
    struct magma_create_semaphore_info tl_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SEMAPHORE_INFO,
            .size = sizeof(tl_info),
        },
        .semaphore_type = MAGMA_SEMAPHORE_TYPE_TIMELINE,
        .use_flags = MAGMA_SEMAPHORE_USE_FLAGS_FENCE,
        .initial_point = 0,
    };
    magma_semaphore_t binary_sem = NULL;
    magma_semaphore_t timeline_sem = NULL;
    magma_status_t status = magma_device_create_semaphore(runner->device, &bin_info, &binary_sem);
    assert(status == MAGMA_STATUS_SUCCESS);
    status = magma_device_create_semaphore(runner->device, &tl_info, &timeline_sem);
    assert(status == MAGMA_STATUS_SUCCESS);

    struct async_submit_args args = {
        .runner = runner,
        .binary_sem = binary_sem,
        .timeline_sem = timeline_sem,
    };
    pthread_t thread;
    int rc = pthread_create(&thread, NULL, async_submit_thread_fn, &args);
    assert(rc == 0);

    // Main thread waits for pending submission and exports BEFORE the background thread submits!
    status = magma_semaphore_timeline_wait(binary_sem, 0, UINT64_MAX,
                                           MAGMA_SEMAPHORE_WAIT_FLAGS_WAIT_PENDING);
    assert(status == MAGMA_STATUS_SUCCESS);

    status = magma_semaphore_wait(binary_sem, UINT64_MAX);
    assert(status == MAGMA_STATUS_SUCCESS);

    struct magma_handle handle;
    memset(&handle, 0, sizeof(handle));
    status = magma_semaphore_export(binary_sem, &handle);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(handle.os_handle >= 0);
    close((int)handle.os_handle);

    status = magma_semaphore_timeline_wait(timeline_sem, 100, UINT64_MAX,
                                           MAGMA_SEMAPHORE_WAIT_FLAGS_WAIT_PENDING);
    assert(status == MAGMA_STATUS_SUCCESS);

    status = magma_semaphore_timeline_wait(timeline_sem, 100, UINT64_MAX, 0);
    assert(status == MAGMA_STATUS_SUCCESS);

    uint64_t val = 0;
    status = magma_semaphore_timeline_query(timeline_sem, &val);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(val >= 100);

    pthread_join(thread, NULL);

    status = magma_semaphore_release(&binary_sem);
    assert(status == MAGMA_STATUS_SUCCESS);
    status = magma_semaphore_release(&timeline_sem);
    assert(status == MAGMA_STATUS_SUCCESS);
    printf("Threaded WAIT_FOR_SUBMIT + export_fence test passed!\n");
}

static void test_buffer_lifecycle_loop(struct test_runner *runner) {
    printf("Testing buffer create/map/export/unmap/release loop (window resize pattern)...\n");
    struct magma_create_semaphore_info bind_sem_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SEMAPHORE_INFO,
            .size = sizeof(bind_sem_info),
        },
        .semaphore_type = MAGMA_SEMAPHORE_TYPE_TIMELINE,
        .use_flags = 0,
        .initial_point = 0,
    };
    magma_semaphore_t bind_sem = NULL;
    magma_status_t status = magma_device_create_semaphore(runner->device, &bind_sem_info, &bind_sem);
    assert(status == MAGMA_STATUS_SUCCESS);
    uint64_t bind_point = 0;

    for (int i = 0; i < 50; i++) {
        struct magma_create_buffer_info create_buffer_info = {
            .header = {
                .s_type = MAGMA_STRUCTURE_TYPE_CREATE_BUFFER_INFO,
                .size = sizeof(create_buffer_info),
            },
            .alignment = 4096,
            .common_flags = MAGMA_BUFFER_FLAG_EXTERNAL | MAGMA_BUFFER_FLAG_SCANOUT,
            .vendor_flags = 0,
            .size = 65536,
        };
        magma_buffer_t temp_buf = NULL;
        status = magma_device_create_buffer(runner->device, &create_buffer_info, &temp_buf);
        assert(status == MAGMA_STATUS_SUCCESS);
        assert(temp_buf != NULL);

        uint64_t temp_va = 0x30000000ULL + (uint64_t)i * 0x10000ULL;
        bind_point++;
        struct magma_submit_map_buffer_info map_info = {
            .header = {
                .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_MAP_BUFFER_INFO,
                .size = sizeof(map_info),
                .p_next = NULL,
            },
            .buffer = temp_buf,
            .buffer_offset = 0,
            .gpu_va = temp_va,
            .size = 65536,
            .flags = MAGMA_GPU_MAP_FLAGS_READ | MAGMA_GPU_MAP_FLAGS_WRITE,
        };
        struct magma_submit_info map_submit = {
            .header = {
                .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_INFO,
                .size = sizeof(map_submit),
                .p_next = &map_info,
            },
            .flags = 0,
            .sync_info = {
                .header = {
                    .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO,
                    .size = sizeof(struct magma_submit_sync_info),
                    .p_next = NULL,
                },
                .num_signal_semaphores = 1,
                .signal_semaphores = &bind_sem,
                .signal_points = &bind_point,
            },
        };
        status = magma_queue_submit(runner->queue, &map_submit);
        assert(status == MAGMA_STATUS_SUCCESS);

        struct magma_mapping mapping;
        memset(&mapping, 0, sizeof(mapping));
        status = magma_buffer_map(temp_buf, &mapping);
        assert(status == MAGMA_STATUS_SUCCESS);
        assert(mapping.ptr != NULL);
        ((uint32_t *)mapping.ptr)[0] = 0x12345678;
        status = magma_buffer_unmap(temp_buf);
        assert(status == MAGMA_STATUS_SUCCESS);

        struct magma_handle handle;
        memset(&handle, 0, sizeof(handle));
        status = magma_buffer_export(temp_buf, &handle);
        assert(status == MAGMA_STATUS_SUCCESS);
        assert(handle.os_handle >= 0);
        close((int)handle.os_handle);

        bind_point++;
        struct magma_submit_unmap_buffer_info unmap_info = {
            .header = {
                .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_UNMAP_BUFFER_INFO,
                .size = sizeof(unmap_info),
                .p_next = NULL,
            },
            .gpu_va = temp_va,
            .size = 65536,
        };
        struct magma_submit_info unmap_submit = {
            .header = {
                .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_INFO,
                .size = sizeof(unmap_submit),
                .p_next = &unmap_info,
            },
            .flags = 0,
            .sync_info = {
                .header = {
                    .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO,
                    .size = sizeof(struct magma_submit_sync_info),
                    .p_next = NULL,
                },
                .num_signal_semaphores = 1,
                .signal_semaphores = &bind_sem,
                .signal_points = &bind_point,
            },
        };
        status = magma_queue_submit(runner->queue, &unmap_submit);
        assert(status == MAGMA_STATUS_SUCCESS);

        status = magma_buffer_release(&temp_buf);
        assert(status == MAGMA_STATUS_SUCCESS);
        assert(temp_buf == NULL);
    }
    status = magma_semaphore_timeline_wait(bind_sem, bind_point, UINT64_MAX, 0);
    assert(status == MAGMA_STATUS_SUCCESS);
    status = magma_semaphore_release(&bind_sem);
    assert(status == MAGMA_STATUS_SUCCESS);
    printf("Completed 50 buffer create/map/export/unmap/release cycles successfully!\n");
}

static void test_submit_resources_info(struct test_runner *runner) {
    enum { NUM_RES = 3 };
    printf("Testing magma_submit_resources_info extension...\n");

    magma_buffer_t res_bufs[NUM_RES] = { NULL };

    struct magma_create_buffer_info create_buffer_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_BUFFER_INFO,
            .size = sizeof(create_buffer_info),
        },
        .alignment = 4096,
        .common_flags = MAGMA_BUFFER_FLAG_CACHED,
        .vendor_flags = 0,
        .size = 4096,
    };
    magma_status_t status;
    for (int i = 0; i < NUM_RES; i++) {
        status = magma_device_create_buffer(runner->device, &create_buffer_info, &res_bufs[i]);
        assert(status == MAGMA_STATUS_SUCCESS);
        assert(res_bufs[i] != NULL);
    }

    // 1. Submit with res_info -> addr_info.
    // Non-Fuchsia backends do not implement MagmaSubmitResourcesInfo and return UNIMPLEMENTED.
    struct magma_submit_address_space_info addr_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_ADDRESS_SPACE_INFO,
            .size = sizeof(addr_info),
            .p_next = NULL,
        },
        .command_va = runner->gpu_va,
        .length = 256,
    };
    struct magma_submit_resources_info res_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_RESOURCES_INFO,
            .size = sizeof(res_info),
            .p_next = &addr_info,
        },
        .num_resources = NUM_RES,
        .buffers = res_bufs,
    };
    struct magma_submit_info submit_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_INFO,
            .size = sizeof(submit_info),
            .p_next = &res_info,
        },
        .flags = 0,
        .sync_info = {
            .header = {
                .s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO,
                .size = sizeof(struct magma_submit_sync_info),
                .p_next = NULL,
            },
        },
    };
    status = magma_queue_submit(runner->queue, &submit_info);
    assert(status == MAGMA_STATUS_UNIMPLEMENTED);

    // 2. Submit with reversed chain order (addr_info -> res_info) to exercise FFI p_next chain.
    res_info.header.p_next = NULL;
    addr_info.header.p_next = &res_info;
    submit_info.header.p_next = &addr_info;
    status = magma_queue_submit(runner->queue, &submit_info);
    assert(status == MAGMA_STATUS_UNIMPLEMENTED);

    for (int i = 0; i < NUM_RES; i++) {
        status = magma_buffer_release(&res_bufs[i]);
        assert(status == MAGMA_STATUS_SUCCESS);
        assert(res_bufs[i] == NULL);
    }

    printf("magma_submit_resources_info extension test passed!\n");
}

static void test_cleanup(struct test_runner *runner) {
    magma_status_t status =
        submit_unmap_buffer(runner->queue, runner->gpu_va, runner->buffer_size);
    assert(status == MAGMA_STATUS_SUCCESS);

    status = magma_buffer_release(&runner->buffer);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->buffer == NULL);

    status = magma_semaphore_release(&runner->semaphore);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->semaphore == NULL);

    status = magma_semaphore_release(&runner->out_semaphore);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->out_semaphore == NULL);

    status = magma_device_release(&runner->device);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->device == NULL);

    for (uint32_t i = 0; i < runner->num_physical_devices; i++) {
        status = magma_physical_device_release(&runner->physical_devices[i]);
        assert(status == MAGMA_STATUS_SUCCESS);
        assert(runner->physical_devices[i] == NULL);
    }
}

int main(void) {
    struct test_runner runner;
    memset(&runner, 0, sizeof(runner));

    test_enumerate_and_init(&runner);
    test_create_resources(&runner);
    test_cpu_buffer_mapping(&runner);
    test_gpu_buffer_mapping(&runner);
    test_semaphore(&runner);
    test_submit_command(&runner);
    test_export_fence(&runner);
    test_export_buffer(&runner);
    test_import_semaphore(&runner);
    test_multiple_submits(&runner);
    test_timeline_semaphore(&runner);
    test_many_semaphores_submit(&runner);
    test_threaded_wait_for_submit(&runner);
    test_buffer_lifecycle_loop(&runner);
    test_submit_resources_info(&runner);
    test_cleanup(&runner);

    printf("\n=== All Magma C-FFI Tests Passed! ===\n");
    return 0;
}
