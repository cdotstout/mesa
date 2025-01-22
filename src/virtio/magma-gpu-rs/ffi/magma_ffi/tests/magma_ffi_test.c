// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

#include <assert.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "magma.h"

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
    uint32_t chosen_mem_type_idx;

    magma_device_t device;
    magma_address_space_t address_space;
    magma_queue_t queue;

    magma_buffer_t buffer;
    uint64_t buffer_size;
    uint64_t gpu_va;

    magma_sync_obj_t sync_obj;
    magma_sync_obj_t out_sync_obj;
};

static void test_enumerate_and_init(struct test_runner *runner) {
    magma_status_t status =
        magma_enumerate_physical_devices(runner->physical_devices, &runner->num_physical_devices);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->num_physical_devices > 0);
    assert(runner->physical_devices[0] != NULL);
    printf("Successfully enumerated %u physical device(s)\n", runner->num_physical_devices);

    magma_get_physical_device_info(runner->physical_devices[0], &runner->dev_info);
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
    struct magma_memory_properties mem_props;
    memset(&mem_props, 0, sizeof(mem_props));
    status = magma_get_memory_properties(runner->physical_devices[0], &mem_props);
    assert(status == MAGMA_STATUS_SUCCESS);

    printf("  Memory heap count: %u\n", mem_props.memory_heap_count);
    for (uint32_t i = 0; i < mem_props.memory_heap_count; i++) {
        printf("    Heap %u: size %lu bytes (0x%lx), flags 0x%08lx\n",
               i,
               (unsigned long)mem_props.memory_heaps[i].heap_size,
               (unsigned long)mem_props.memory_heaps[i].heap_size,
               (unsigned long)mem_props.memory_heaps[i].heap_flags);
    }
    printf("  Memory type count: %u\n", mem_props.memory_type_count);
    for (uint32_t i = 0; i < mem_props.memory_type_count; i++) {
        printf("    Memory type %u: heap_idx %u, property_flags 0x%08x\n",
               i,
               mem_props.memory_types[i].heap_idx,
               mem_props.memory_types[i].property_flags);
    }

    runner->chosen_mem_type_idx = 0;
    for (uint32_t i = 0; i < mem_props.memory_type_count; i++) {
        uint32_t flags = mem_props.memory_types[i].property_flags;
        if ((flags & MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT) &&
            (flags & MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT)) {
            runner->chosen_mem_type_idx = i;
            break;
        }
    }

    free(mem_props.memory_types);
    free(mem_props.memory_heaps);

    status = magma_create_device(runner->physical_devices[0], &runner->device);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->device != NULL);
    printf("Successfully created device\n");

    // Query memory budget for heap 0
    struct magma_heap_budget budget;
    memset(&budget, 0, sizeof(budget));
    status = magma_get_memory_budget(runner->device, 0, &budget);
    assert(status == MAGMA_STATUS_SUCCESS);
    printf("Heap 0 budget: %lu bytes, usage: %lu bytes\n",
           (unsigned long)budget.budget, (unsigned long)budget.usage);
}

static void test_create_resources(struct test_runner *runner) {
    magma_status_t status = magma_create_address_space(runner->device, &runner->address_space);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->address_space != NULL);

    struct magma_create_queue_info queue_info = {
        .queue_family_idx = 0,
        .priority = 0,
    };
    status = magma_create_queue(runner->device, runner->address_space, &queue_info, &runner->queue);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->queue != NULL);

    runner->buffer_size = 4096;
    runner->gpu_va = 0x20000000;
    struct magma_create_buffer_info create_buffer_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_BUFFER_INFO,
            .size = sizeof(create_buffer_info),
        },
        .memory_type_idx = runner->chosen_mem_type_idx,
        .alignment = 4096,
        .common_flags = 0,
        .vendor_flags = 0,
        .size = runner->buffer_size,
    };
    status = magma_create_buffer(runner->device, &create_buffer_info, &runner->buffer);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->buffer != NULL);
}

static void test_cpu_buffer_mapping(struct test_runner *runner) {
    struct magma_mapping mapping;
    memset(&mapping, 0, sizeof(mapping));
    magma_status_t status = magma_map_buffer_cpu(runner->buffer, &mapping);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(mapping.ptr != NULL);
    assert(mapping.size >= runner->buffer_size);

    uint32_t noop_cmd = get_vendor_noop_cmd(runner->dev_info.vendor_id);
    uint32_t *words = (uint32_t*)mapping.ptr;
    for (size_t i = 0; i < mapping.size / 4; i++) {
        words[i] = noop_cmd;
    }

    status = magma_unmap_buffer_cpu(runner->buffer);
    assert(status == MAGMA_STATUS_SUCCESS);
}

static void test_gpu_buffer_mapping(struct test_runner *runner) {
    magma_status_t status = magma_map_buffer_gpu(
        runner->address_space, runner->buffer, 0, runner->gpu_va, runner->buffer_size,
        MAGMA_GPU_MAP_FLAGS_READ | MAGMA_GPU_MAP_FLAGS_WRITE | MAGMA_GPU_MAP_FLAGS_EXECUTE);
    assert(status == MAGMA_STATUS_SUCCESS);
}

static void test_sync_obj(struct test_runner *runner) {
    struct magma_create_sync_obj_info sync_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SYNC_OBJ_INFO,
            .size = sizeof(sync_info),
        },
        .sync_type = MAGMA_SYNC_OBJ_TYPE_BINARY,
        .use_flags = MAGMA_SYNC_OBJ_USE_FLAGS_FENCE,
        .initial_point = 0,
    };
    magma_status_t status = magma_create_sync_obj(runner->device, &sync_info, &runner->sync_obj);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->sync_obj != NULL);

    status = magma_sync_obj_signal(runner->sync_obj);
    assert(status == MAGMA_STATUS_SUCCESS);

    status = magma_sync_obj_wait(runner->sync_obj, 1000000000ULL);
    assert(status == MAGMA_STATUS_SUCCESS);
}

static void test_submit_command(struct test_runner *runner) {
    struct magma_create_sync_obj_info sync_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SYNC_OBJ_INFO,
            .size = sizeof(sync_info),
        },
        .sync_type = MAGMA_SYNC_OBJ_TYPE_BINARY,
        .use_flags = MAGMA_SYNC_OBJ_USE_FLAGS_EXPORTABLE,
        .initial_point = 0,
    };
    magma_status_t status = magma_create_sync_obj(runner->device, &sync_info, &runner->out_sync_obj);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->out_sync_obj != NULL);

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
    submit_info.sync_info.num_wait_sync_objs = 1;
    submit_info.sync_info.wait_sync_objs[0] = runner->sync_obj;
    submit_info.sync_info.num_signal_sync_objs = 1;
    submit_info.sync_info.signal_sync_objs[0] = runner->out_sync_obj;

    status = magma_submit_command(runner->queue, &submit_info);
    assert(status == MAGMA_STATUS_SUCCESS);
}

static void test_export_fence(struct test_runner *runner) {
    struct magma_handle handle;
    memset(&handle, 0, sizeof(handle));
    magma_status_t status = magma_sync_obj_export(runner->out_sync_obj, &handle);
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
    close((int)handle.os_handle);
}

static void test_import_sync_obj(struct test_runner *runner) {
    struct magma_handle handle;
    memset(&handle, 0, sizeof(handle));
    magma_status_t status = magma_sync_obj_export(runner->out_sync_obj, &handle);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(handle.os_handle >= 0);

    struct magma_create_sync_obj_info sync_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SYNC_OBJ_INFO,
            .size = sizeof(sync_info),
        },
        .sync_type = MAGMA_SYNC_OBJ_TYPE_BINARY,
        .use_flags = MAGMA_SYNC_OBJ_USE_FLAGS_FENCE,
        .initial_point = 0,
    };
    magma_sync_obj_t imported_sync_obj = NULL;
    status = magma_create_sync_obj(runner->device, &sync_info, &imported_sync_obj);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(imported_sync_obj != NULL);

    status = magma_sync_obj_import(imported_sync_obj, &handle);
    assert(status == MAGMA_STATUS_SUCCESS);
    printf("Successfully imported sync object: fd=%d\n", (int)handle.os_handle);

    // Submit a command waiting on the imported sync object to verify guest-side fence wait.
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
    submit_info.sync_info.num_wait_sync_objs = 1;
    submit_info.sync_info.wait_sync_objs[0] = imported_sync_obj;
    submit_info.sync_info.num_signal_sync_objs = 0;

    status = magma_submit_command(runner->queue, &submit_info);
    assert(status == MAGMA_STATUS_SUCCESS);
    printf("Successfully submitted command waiting on imported sync object!\n");

    status = magma_sync_obj_close(&imported_sync_obj);
    assert(status == MAGMA_STATUS_SUCCESS);
}

static void test_multiple_submits(struct test_runner *runner) {
    printf("Testing multiple submits with fresh sync objects...\n");
    for (int i = 0; i < 20; i++) {
        struct magma_create_sync_obj_info sync_info = {
            .header = {
                .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SYNC_OBJ_INFO,
                .size = sizeof(sync_info),
            },
            .sync_type = MAGMA_SYNC_OBJ_TYPE_BINARY,
            .use_flags = MAGMA_SYNC_OBJ_USE_FLAGS_FENCE | MAGMA_SYNC_OBJ_USE_FLAGS_EXPORTABLE,
            .initial_point = 0,
        };
        magma_sync_obj_t wait_so = NULL;
        magma_sync_obj_t sig_so = NULL;
        magma_status_t status = magma_create_sync_obj(runner->device, &sync_info, &wait_so);
        assert(status == MAGMA_STATUS_SUCCESS);
        status = magma_sync_obj_signal(wait_so);
        assert(status == MAGMA_STATUS_SUCCESS);
        status = magma_create_sync_obj(runner->device, &sync_info, &sig_so);
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
        submit_info.sync_info.num_wait_sync_objs = 1;
        submit_info.sync_info.wait_sync_objs[0] = wait_so;
        submit_info.sync_info.num_signal_sync_objs = 1;
        submit_info.sync_info.signal_sync_objs[0] = sig_so;

        status = magma_submit_command(runner->queue, &submit_info);
        assert(status == MAGMA_STATUS_SUCCESS);

        status = magma_sync_obj_close(&wait_so);
        assert(status == MAGMA_STATUS_SUCCESS);
        status = magma_sync_obj_close(&sig_so);
        assert(status == MAGMA_STATUS_SUCCESS);
    }
    printf("Completed 20 submits successfully!\n");
}

static void test_timeline_sync_obj(struct test_runner *runner) {
    printf("Testing timeline sync objects (CPU signal + GPU wait/signal + query)...\n");
    struct magma_create_sync_obj_info sync_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SYNC_OBJ_INFO,
            .size = sizeof(sync_info),
        },
        .sync_type = MAGMA_SYNC_OBJ_TYPE_TIMELINE,
        .use_flags = MAGMA_SYNC_OBJ_USE_FLAGS_FENCE,
        .initial_point = 0,
    };
    magma_sync_obj_t timeline_in = NULL;
    magma_sync_obj_t timeline_out = NULL;
    magma_status_t status = magma_create_sync_obj(runner->device, &sync_info, &timeline_in);
    assert(status == MAGMA_STATUS_SUCCESS);
    status = magma_create_sync_obj(runner->device, &sync_info, &timeline_out);
    assert(status == MAGMA_STATUS_SUCCESS);

    // 1. CPU timeline signal, query, and wait
    status = magma_sync_obj_timeline_signal(timeline_in, 5);
    assert(status == MAGMA_STATUS_SUCCESS);

    uint64_t val = 0;
    status = magma_sync_obj_timeline_query(timeline_in, &val);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(val == 5);

    status = magma_sync_obj_timeline_wait(timeline_in, 5, UINT64_MAX, 0);
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

    submit_info.sync_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO;
    submit_info.sync_info.header.size = sizeof(submit_info.sync_info);
    submit_info.sync_info.num_wait_sync_objs = 1;
    submit_info.sync_info.wait_sync_objs[0] = timeline_in;
    submit_info.sync_info.wait_points[0] = 5;
    submit_info.sync_info.num_signal_sync_objs = 1;
    submit_info.sync_info.signal_sync_objs[0] = timeline_out;
    submit_info.sync_info.signal_points[0] = 42;

    status = magma_submit_command(runner->queue, &submit_info);
    assert(status == MAGMA_STATUS_SUCCESS);

    status = magma_sync_obj_timeline_wait(timeline_out, 42, UINT64_MAX, 0);
    assert(status == MAGMA_STATUS_SUCCESS);

    val = 0;
    status = magma_sync_obj_timeline_query(timeline_out, &val);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(val >= 42);

    status = magma_sync_obj_close(&timeline_in);
    assert(status == MAGMA_STATUS_SUCCESS);
    status = magma_sync_obj_close(&timeline_out);
    assert(status == MAGMA_STATUS_SUCCESS);
    printf("Timeline sync object test passed!\n");
}

struct async_submit_args {
    struct test_runner *runner;
    magma_sync_obj_t binary_so;
    magma_sync_obj_t timeline_so;
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

    submit_info.sync_info.header.s_type = MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO;
    submit_info.sync_info.header.size = sizeof(submit_info.sync_info);
    submit_info.sync_info.num_wait_sync_objs = 0;
    submit_info.sync_info.num_signal_sync_objs = 2;
    submit_info.sync_info.signal_sync_objs[0] = args->binary_so;
    submit_info.sync_info.signal_points[0] = 0;
    submit_info.sync_info.signal_sync_objs[1] = args->timeline_so;
    submit_info.sync_info.signal_points[1] = 100;

    magma_status_t status = magma_submit_command(args->runner->queue, &submit_info);
    assert(status == MAGMA_STATUS_SUCCESS);
    return NULL;
}

static void test_threaded_wait_for_submit(struct test_runner *runner) {
    printf("Testing threaded WAIT_FOR_SUBMIT + export_fence (vk_queue_submit_thread pattern)...\n");
    struct magma_create_sync_obj_info bin_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SYNC_OBJ_INFO,
            .size = sizeof(bin_info),
        },
        .sync_type = MAGMA_SYNC_OBJ_TYPE_BINARY,
        .use_flags = MAGMA_SYNC_OBJ_USE_FLAGS_EXPORTABLE,
        .initial_point = 0,
    };
    struct magma_create_sync_obj_info tl_info = {
        .header = {
            .s_type = MAGMA_STRUCTURE_TYPE_CREATE_SYNC_OBJ_INFO,
            .size = sizeof(tl_info),
        },
        .sync_type = MAGMA_SYNC_OBJ_TYPE_TIMELINE,
        .use_flags = MAGMA_SYNC_OBJ_USE_FLAGS_FENCE,
        .initial_point = 0,
    };
    magma_sync_obj_t binary_so = NULL;
    magma_sync_obj_t timeline_so = NULL;
    magma_status_t status = magma_create_sync_obj(runner->device, &bin_info, &binary_so);
    assert(status == MAGMA_STATUS_SUCCESS);
    status = magma_create_sync_obj(runner->device, &tl_info, &timeline_so);
    assert(status == MAGMA_STATUS_SUCCESS);

    struct async_submit_args args = {
        .runner = runner,
        .binary_so = binary_so,
        .timeline_so = timeline_so,
    };
    pthread_t thread;
    int rc = pthread_create(&thread, NULL, async_submit_thread_fn, &args);
    assert(rc == 0);

    // Main thread waits and exports BEFORE the background thread submits!
    status = magma_sync_obj_wait(binary_so, UINT64_MAX);
    assert(status == MAGMA_STATUS_SUCCESS);

    struct magma_handle handle;
    memset(&handle, 0, sizeof(handle));
    status = magma_sync_obj_export(binary_so, &handle);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(handle.os_handle >= 0);
    close((int)handle.os_handle);

    status = magma_sync_obj_timeline_wait(timeline_so, 100, UINT64_MAX, 0);
    assert(status == MAGMA_STATUS_SUCCESS);

    uint64_t val = 0;
    status = magma_sync_obj_timeline_query(timeline_so, &val);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(val >= 100);

    pthread_join(thread, NULL);

    status = magma_sync_obj_close(&binary_so);
    assert(status == MAGMA_STATUS_SUCCESS);
    status = magma_sync_obj_close(&timeline_so);
    assert(status == MAGMA_STATUS_SUCCESS);
    printf("Threaded WAIT_FOR_SUBMIT + export_fence test passed!\n");
}

static void test_buffer_lifecycle_loop(struct test_runner *runner) {
    printf("Testing buffer create/map/export/unmap/close loop (window resize pattern)...\n");
    for (int i = 0; i < 50; i++) {
        struct magma_create_buffer_info create_buffer_info = {
            .header = {
                .s_type = MAGMA_STRUCTURE_TYPE_CREATE_BUFFER_INFO,
                .size = sizeof(create_buffer_info),
            },
            .memory_type_idx = runner->chosen_mem_type_idx,
            .alignment = 4096,
            .common_flags = MAGMA_BUFFER_FLAG_EXTERNAL | MAGMA_BUFFER_FLAG_SCANOUT,
            .vendor_flags = 0,
            .size = 65536,
        };
        magma_buffer_t temp_buf = NULL;
        magma_status_t status = magma_create_buffer(runner->device, &create_buffer_info, &temp_buf);
        assert(status == MAGMA_STATUS_SUCCESS);
        assert(temp_buf != NULL);

        uint64_t temp_va = 0x30000000ULL + (uint64_t)i * 0x10000ULL;
        status = magma_map_buffer_gpu(
            runner->address_space, temp_buf, 0, temp_va, 65536,
            MAGMA_GPU_MAP_FLAGS_READ | MAGMA_GPU_MAP_FLAGS_WRITE);
        assert(status == MAGMA_STATUS_SUCCESS);

        struct magma_mapping mapping;
        memset(&mapping, 0, sizeof(mapping));
        status = magma_map_buffer_cpu(temp_buf, &mapping);
        assert(status == MAGMA_STATUS_SUCCESS);
        assert(mapping.ptr != NULL);
        ((uint32_t *)mapping.ptr)[0] = 0x12345678;
        status = magma_unmap_buffer_cpu(temp_buf);
        assert(status == MAGMA_STATUS_SUCCESS);

        struct magma_handle handle;
        memset(&handle, 0, sizeof(handle));
        status = magma_buffer_export(temp_buf, &handle);
        assert(status == MAGMA_STATUS_SUCCESS);
        assert(handle.os_handle >= 0);
        close((int)handle.os_handle);

        status = magma_unmap_buffer_gpu(runner->address_space, temp_va, 65536);
        assert(status == MAGMA_STATUS_SUCCESS);

        status = magma_buffer_close(&temp_buf);
        assert(status == MAGMA_STATUS_SUCCESS);
        assert(temp_buf == NULL);
    }
    printf("Completed 50 buffer create/map/export/unmap/close cycles successfully!\n");
}

static void test_cleanup(struct test_runner *runner) {
    magma_status_t status =
        magma_unmap_buffer_gpu(runner->address_space, runner->gpu_va, runner->buffer_size);
    assert(status == MAGMA_STATUS_SUCCESS);

    status = magma_buffer_close(&runner->buffer);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->buffer == NULL);

    status = magma_sync_obj_close(&runner->sync_obj);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->sync_obj == NULL);

    status = magma_sync_obj_close(&runner->out_sync_obj);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->out_sync_obj == NULL);

    status = magma_device_close(&runner->device);
    assert(status == MAGMA_STATUS_SUCCESS);
    assert(runner->device == NULL);

    for (uint32_t i = 0; i < runner->num_physical_devices; i++) {
        status = magma_physical_device_close(&runner->physical_devices[i]);
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
    test_sync_obj(&runner);
    test_submit_command(&runner);
    test_export_fence(&runner);
    test_export_buffer(&runner);
    test_import_sync_obj(&runner);
    test_multiple_submits(&runner);
    test_timeline_sync_obj(&runner);
    test_threaded_wait_for_submit(&runner);
    test_buffer_lifecycle_loop(&runner);
    test_cleanup(&runner);

    printf("\n=== All Magma C-FFI Tests Passed! ===\n");
    return 0;
}
