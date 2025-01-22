// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT
//
// Generated via:
//   https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/magma/gorgonzola
//
// Submit patches, do not hand-edit.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::boxed::Box;
use std::convert::TryInto;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr::null_mut;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use std::sync::{Arc, Mutex};

use libc::{EINVAL, ESRCH};
use log::{debug, error};

use magma_gpu_magma::AddressSpace;
use magma_gpu_magma::Buffer;
use magma_gpu_magma::Device;
use magma_gpu_magma::MagmaBufferFlag;
use magma_gpu_magma::MagmaCreateBufferInfo;
use magma_gpu_magma::MagmaCreateQueueInfo;
use magma_gpu_magma::MagmaCreateSyncObjInfo;
use magma_gpu_magma::MagmaDeviceCreateInfo;
use magma_gpu_magma::MagmaGpuMapFlags;
use magma_gpu_magma::MagmaHandle;
use magma_gpu_magma::MagmaHandleType;
use magma_gpu_magma::MagmaHeap;
use magma_gpu_magma::MagmaHeapBudget;
use magma_gpu_magma::MagmaMapping;
use magma_gpu_magma::MagmaMemoryProperties;
use magma_gpu_magma::MagmaMemoryProperty;
use magma_gpu_magma::MagmaMemoryType;
use magma_gpu_magma::MagmaPciBusInfo;
use magma_gpu_magma::MagmaPhysicalDeviceInfo;
use magma_gpu_magma::MagmaPhysicalDeviceMemoryHeap;
use magma_gpu_magma::MagmaPhysicalDeviceMemoryType;
use magma_gpu_magma::MagmaQueueFamilyProperties;
use magma_gpu_magma::MagmaQueueFlags;
use magma_gpu_magma::MagmaStructureType;
use magma_gpu_magma::MagmaStructureTypeHeader;
use magma_gpu_magma::MagmaSubmitAddressSpaceInfo;
use magma_gpu_magma::MagmaSubmitBufferInfo;
use magma_gpu_magma::MagmaSubmitInfo;
use magma_gpu_magma::MagmaSubmitSyncInfo;
use magma_gpu_magma::MagmaSyncCapability;
use magma_gpu_magma::MagmaSyncObjType;
use magma_gpu_magma::MagmaSyncObjUseFlags;
use magma_gpu_magma::MagmaSyncProperties;
use magma_gpu_magma::PhysicalDevice;
use magma_gpu_magma::Queue;
use magma_gpu_magma::SyncObj;
use magma_gpu_magma::{Error, Result};

const NO_ERROR: i32 = 0;

fn return_result(result: Result<()>) -> i32 {
    match result {
        Ok(()) => NO_ERROR,
        Err(e) => {
            debug!("error: {:?}", e);
            e.into()
        }
    }
}

macro_rules! return_on_error {
    ($result:expr) => {
        match $result {
            Ok(t) => t,
            Err(e) => {
                debug!("error: {:?}", e);
                return e.into();
            }
        }
    };
}

const MAGMA_MAX_SYNCOBJS: usize = 16;
const MAGMA_MAX_PHYSICAL_DEVICES: usize = 8;
const MAGMA_MAX_MEMORY_HEAPS: usize = 32;
const MAGMA_MAX_MEMORY_TYPES: usize = 16;
const MAGMA_MAX_QUEUES: usize = 16;
#[allow(non_camel_case_types)]
pub type magma_queue = Queue;

#[allow(non_camel_case_types)]
pub type magma_address_space = AddressSpace;

#[allow(non_camel_case_types)]
pub type magma_buffer = Buffer;

#[allow(non_camel_case_types)]
pub type magma_sync_obj = SyncObj;

#[allow(non_camel_case_types)]
pub type magma_physical_device = PhysicalDevice;

#[allow(non_camel_case_types)]
pub type magma_device = Device;

#[allow(non_camel_case_types)]
pub type magma_sync_obj_type = MagmaSyncObjType;

#[allow(non_camel_case_types)]
pub type magma_memory_property = MagmaMemoryProperty;

#[allow(non_camel_case_types)]
pub type magma_queue_flags = MagmaQueueFlags;

#[allow(non_camel_case_types)]
pub type magma_gpu_map_flags = MagmaGpuMapFlags;

#[allow(non_camel_case_types)]
pub type magma_buffer_flag = MagmaBufferFlag;

#[allow(non_camel_case_types)]
pub type magma_sync_capability = MagmaSyncCapability;

#[allow(non_camel_case_types)]
pub type magma_sync_obj_use_flags = MagmaSyncObjUseFlags;

#[allow(non_camel_case_types)]
pub type magma_handle_type = MagmaHandleType;

#[allow(non_camel_case_types)]
pub type magma_heap = MagmaHeap;

#[allow(non_camel_case_types)]
pub type magma_heap_budget = MagmaHeapBudget;

#[allow(non_camel_case_types)]
pub type magma_memory_type = MagmaMemoryType;

#[allow(non_camel_case_types)]
pub type magma_memory_properties = MagmaMemoryProperties;

#[allow(non_camel_case_types)]
pub type magma_pci_bus_info = MagmaPciBusInfo;

#[allow(non_camel_case_types)]
pub type magma_create_queue_info = MagmaCreateQueueInfo;

#[allow(non_camel_case_types)]
pub type magma_queue_family_properties = MagmaQueueFamilyProperties;

#[allow(non_camel_case_types)]
pub type magma_mapping = MagmaMapping;

#[allow(non_camel_case_types)]
pub type magma_handle = MagmaHandle;

#[allow(non_camel_case_types)]
pub type magma_create_buffer_info = MagmaCreateBufferInfo;

#[allow(non_camel_case_types)]
pub type magma_device_create_info = MagmaDeviceCreateInfo;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct magma_submit_sync_info {
    pub header: MagmaStructureTypeHeader,
    pub num_wait_sync_objs: u32,
    pub num_signal_sync_objs: u32,
    pub wait_sync_objs: [*mut magma_sync_obj; MAGMA_MAX_SYNCOBJS],
    pub signal_sync_objs: [*mut magma_sync_obj; MAGMA_MAX_SYNCOBJS],
    pub wait_points: [u64; MAGMA_MAX_SYNCOBJS],
    pub signal_points: [u64; MAGMA_MAX_SYNCOBJS],
}

impl magma_submit_sync_info {
    pub fn to_rust(&self) -> MagmaSubmitSyncInfo {
        let mut wait_sync_objs: [Option<SyncObj>; MAGMA_MAX_SYNCOBJS] = Default::default();
        let limit_wait_sync_objs = (self.num_wait_sync_objs as usize).min(MAGMA_MAX_SYNCOBJS);
        for i in 0..limit_wait_sync_objs {
            let ptr = self.wait_sync_objs[i];
            if !ptr.is_null() {
                wait_sync_objs[i] = Some(unsafe { (*ptr).clone() });
            }
        }
        let mut signal_sync_objs: [Option<SyncObj>; MAGMA_MAX_SYNCOBJS] = Default::default();
        let limit_signal_sync_objs = (self.num_signal_sync_objs as usize).min(MAGMA_MAX_SYNCOBJS);
        for i in 0..limit_signal_sync_objs {
            let ptr = self.signal_sync_objs[i];
            if !ptr.is_null() {
                signal_sync_objs[i] = Some(unsafe { (*ptr).clone() });
            }
        }
        MagmaSubmitSyncInfo {
            header: MagmaStructureTypeHeader {
                stype: self.header.stype,
                size: std::mem::size_of::<MagmaSubmitSyncInfo>() as u32,
                p_next: self.header.p_next,
            },
            num_wait_sync_objs: self.num_wait_sync_objs,
            num_signal_sync_objs: self.num_signal_sync_objs,
            wait_sync_objs,
            signal_sync_objs,
            wait_points: self.wait_points,
            signal_points: self.signal_points,
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct magma_submit_info {
    pub header: MagmaStructureTypeHeader,
    pub flags: u32,
    pub sync_info: magma_submit_sync_info,
}

impl magma_submit_info {
    pub fn to_rust(&self) -> MagmaSubmitInfo {
        MagmaSubmitInfo {
            header: MagmaStructureTypeHeader {
                stype: self.header.stype,
                size: std::mem::size_of::<MagmaSubmitInfo>() as u32,
                p_next: self.header.p_next,
            },
            flags: self.flags,
            sync_info: self.sync_info.to_rust(),
            ..Default::default()
        }
    }
}

#[allow(non_camel_case_types)]
pub type magma_submit_address_space_info = MagmaSubmitAddressSpaceInfo;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct magma_submit_buffer_info {
    pub header: MagmaStructureTypeHeader,
    pub command_buffer: *mut magma_buffer,
    pub start_offset: u64,
    pub length: u64,
}

impl magma_submit_buffer_info {
    pub fn to_rust(&self) -> MagmaSubmitBufferInfo {
        let command_buffer = 0;
        MagmaSubmitBufferInfo {
            header: MagmaStructureTypeHeader {
                stype: self.header.stype,
                size: std::mem::size_of::<MagmaSubmitBufferInfo>() as u32,
                p_next: self.header.p_next,
            },
            command_buffer,
            start_offset: self.start_offset,
            length: self.length,
            ..Default::default()
        }
    }
}

#[allow(non_camel_case_types)]
pub type magma_create_sync_obj_info = MagmaCreateSyncObjInfo;

#[allow(non_camel_case_types)]
pub type magma_sync_properties = MagmaSyncProperties;

#[allow(non_camel_case_types)]
pub type magma_physical_device_info = MagmaPhysicalDeviceInfo;

#[allow(non_camel_case_types)]
pub type magma_physical_device_memory_type = MagmaPhysicalDeviceMemoryType;

#[allow(non_camel_case_types)]
pub type magma_physical_device_memory_heap = MagmaPhysicalDeviceMemoryHeap;

#[no_mangle]
pub unsafe extern "C" fn magma_create_device(
    physical_device: &mut magma_physical_device,
    device: &mut *mut magma_device,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_create_device");
        let result = physical_device.create_device();
        let res = return_on_error!(result);
        *device = Box::into_raw(Box::new(res)) as _;
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_get_memory_budget(
    device: &mut magma_device,
    heap_idx: u32,
    budget: &mut magma_heap_budget,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_get_memory_budget");
        let result = device.get_memory_budget(heap_idx);
        *budget = return_on_error!(result);
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_create_buffer(
    device: &mut magma_device,
    info: &magma_create_buffer_info,
    buffer_out: &mut *mut magma_buffer,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_create_buffer");
        let result = device.create_buffer(info);
        let res = return_on_error!(result);
        *buffer_out = Box::into_raw(Box::new(res)) as _;
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_create_address_space(
    device: &mut magma_device,
    address_space: &mut *mut magma_address_space,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_create_address_space");
        let result = device.create_address_space();
        let res = return_on_error!(result);
        *address_space = Box::into_raw(Box::new(res)) as _;
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_create_queue(
    device: &mut magma_device,
    address_space: &mut magma_address_space,
    info: &magma_create_queue_info,
    queue: &mut *mut magma_queue,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_create_queue");
        let result = device.create_queue(address_space, info);
        let res = return_on_error!(result);
        *queue = Box::into_raw(Box::new(res)) as _;
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_device_close(device: &mut *mut magma_device) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_device_close");
        if !(*device).is_null() {
            let _ = unsafe { Box::from_raw(*device) };
            *device = null_mut();
        }
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_physical_device_close(
    physical_device: &mut *mut magma_physical_device,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_physical_device_close");
        if !(*physical_device).is_null() {
            let _ = unsafe { Box::from_raw(*physical_device) };
            *physical_device = null_mut();
        }
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_buffer_close(buffer: &mut *mut magma_buffer) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_buffer_close");
        if !(*buffer).is_null() {
            let _ = unsafe { Box::from_raw(*buffer) };
            *buffer = null_mut();
        }
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_queue_close(queue: &mut *mut magma_queue) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_queue_close");
        if !(*queue).is_null() {
            let _ = unsafe { Box::from_raw(*queue) };
            *queue = null_mut();
        }
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_address_space_close(
    address_space: &mut *mut magma_address_space,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_address_space_close");
        if !(*address_space).is_null() {
            let _ = unsafe { Box::from_raw(*address_space) };
            *address_space = null_mut();
        }
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_map_buffer_gpu(
    address_space: &mut magma_address_space,
    buffer: &mut magma_buffer,
    buffer_offset: u64,
    gpu_va: u64,
    size: u64,
    flags: magma_gpu_map_flags,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_map_buffer_gpu");
        let result = address_space.map_buffer_gpu(buffer, buffer_offset, gpu_va, size, flags);
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_unmap_buffer_gpu(
    address_space: &mut magma_address_space,
    gpu_va: u64,
    size: u64,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_unmap_buffer_gpu");
        let result = address_space.unmap_buffer_gpu(gpu_va, size);
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_submit_command(
    queue: &mut magma_queue,
    submit_info: &magma_submit_info,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_submit_command");
        let rust_submit_info = submit_info.to_rust();
        let result = queue.submit_command(&rust_submit_info);
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_create_sync_obj(
    device: &mut magma_device,
    info: &magma_create_sync_obj_info,
    sync_obj: &mut *mut magma_sync_obj,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_create_sync_obj");
        let result = device.create_sync_obj(info);
        let res = return_on_error!(result);
        *sync_obj = Box::into_raw(Box::new(res)) as _;
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_sync_obj_close(sync_obj: &mut *mut magma_sync_obj) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_sync_obj_close");
        if !(*sync_obj).is_null() {
            let _ = unsafe { Box::from_raw(*sync_obj) };
            *sync_obj = null_mut();
        }
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_sync_obj_wait(
    sync_obj: &mut magma_sync_obj,
    timeout_ns: u64,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_sync_obj_wait");
        let result = sync_obj.wait(timeout_ns);
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_sync_obj_signal(sync_obj: &mut magma_sync_obj) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_sync_obj_signal");
        let result = sync_obj.signal();
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_sync_obj_timeline_wait(
    sync_obj: &mut magma_sync_obj,
    point: u64,
    timeout_ns: u64,
    flags: u32,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_sync_obj_timeline_wait");
        let result = sync_obj.timeline_wait(point, timeout_ns, flags);
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_sync_obj_timeline_signal(
    sync_obj: &mut magma_sync_obj,
    point: u64,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_sync_obj_timeline_signal");
        let result = sync_obj.timeline_signal(point);
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_sync_obj_timeline_query(
    sync_obj: &mut magma_sync_obj,
    point_out: &mut u64,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_sync_obj_timeline_query");
        let result = sync_obj.timeline_query();
        *point_out = return_on_error!(result);
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_get_physical_device_info(
    physical_device: &mut magma_physical_device,
    info: &mut magma_physical_device_info,
) -> () {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_get_physical_device_info");
        let result = physical_device.info();
        *info = *result;
    }))
    .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn magma_queue_check_status(queue: &mut magma_queue) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_queue_check_status");
        let result = queue.check_status();
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_map_buffer_cpu(
    buffer: &mut magma_buffer,
    mapping_out: &mut magma_mapping,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_map_buffer_cpu");
        let result = buffer.map_cpu();
        *mapping_out = return_on_error!(result);
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_unmap_buffer_cpu(buffer: &mut magma_buffer) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_unmap_buffer_cpu");
        let result = buffer.unmap_cpu();
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}
