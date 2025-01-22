// Copyright 2026 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::boxed::Box;
use std::panic::catch_unwind;
use std::panic::AssertUnwindSafe;
use std::slice::from_raw_parts_mut;

use libc::{EINVAL, ESRCH};
use log::error;

use magma_gpu_magma::enumerate_devices;
use magma_gpu_magma::PhysicalDevice;

const NO_ERROR: i32 = 0;
const MAGMA_MAX_PHYSICAL_DEVICES: u32 = 8;

macro_rules! return_on_error {
    ($result:expr) => {
        match $result {
            Ok(t) => t,
            Err(e) => {
                error!("An error occurred: {}", e);
                return e.into();
            }
        }
    };
}

#[allow(non_camel_case_types)]
type magma_physical_device = PhysicalDevice;

// The following structs (in define.rs) must be ABI-compatible with FFI header
// (magma.h).

#[no_mangle]
pub unsafe extern "C" fn magma_enumerate_physical_devices(
    physical_devices: &mut *mut magma_physical_device,
    num_devices: &mut u32,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let result = enumerate_devices();
        let phys_devs = return_on_error!(result);
        *num_devices = phys_devs.len().try_into().unwrap();
        if *num_devices > MAGMA_MAX_PHYSICAL_DEVICES {
            return -EINVAL;
        }

        let physical_devices =
            from_raw_parts_mut(physical_devices, MAGMA_MAX_PHYSICAL_DEVICES as usize);
        for (i, phys_dev) in phys_devs.into_iter().enumerate() {
            physical_devices[i] = Box::into_raw(Box::new(phys_dev)) as _;
        }

        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

use magma_gpu::util::FromRawDescriptor;
use magma_gpu::util::Handle as MagmaGpuHandle;
use magma_gpu::util::IntoRawDescriptor;
use magma_gpu::util::OwnedDescriptor;
use magma_gpu_magma::Buffer;
use magma_gpu_magma::Device;
use magma_gpu_magma::MagmaHandle;
use magma_gpu_magma::MagmaImportHandleInfo;
use magma_gpu_magma::MagmaSyncCapability;
use magma_gpu_magma::MagmaSyncProperties;
use magma_gpu_magma::SyncObj;
use std::ffi::c_void;

#[allow(non_camel_case_types)]
type magma_device = Device;

#[allow(non_camel_case_types)]
type magma_buffer = Buffer;

#[allow(non_camel_case_types)]
type magma_sync_obj = SyncObj;

#[allow(non_camel_case_types)]
type magma_handle = MagmaHandle;

#[allow(non_camel_case_types)]
type magma_sync_properties = MagmaSyncProperties;

#[no_mangle]
pub unsafe extern "C" fn magma_sync_obj_export(
    sync_obj: &mut magma_sync_obj,
    handle_out: &mut magma_handle,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let result = sync_obj.export_fence();
        let handle = return_on_error!(result);
        *handle_out = MagmaHandle {
            os_handle: handle.os_handle.into_raw_descriptor() as i64,
            handle_type: handle.handle_type,
        };
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_sync_obj_import(
    sync_obj: &mut magma_sync_obj,
    handle: &magma_handle,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let descriptor = OwnedDescriptor::from_raw_descriptor(handle.os_handle as i32);
        let gpu_handle = MagmaGpuHandle {
            os_handle: descriptor,
            handle_type: handle.handle_type,
        };
        let result = sync_obj.import(gpu_handle);
        return_on_error!(result);
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_buffer_export(
    buffer: &mut magma_buffer,
    handle_out: &mut magma_handle,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let result = buffer.export();
        let handle = return_on_error!(result);
        *handle_out = MagmaHandle {
            os_handle: handle.os_handle.into_raw_descriptor() as i64,
            handle_type: handle.handle_type,
        };
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_buffer_import(
    device: &mut magma_device,
    handle: &magma_handle,
    buffer_out: &mut *mut magma_buffer,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let descriptor = OwnedDescriptor::from_raw_descriptor(handle.os_handle as i32);
        let gpu_handle = MagmaGpuHandle {
            os_handle: descriptor,
            handle_type: handle.handle_type,
        };
        let import_info = MagmaImportHandleInfo {
            handle: gpu_handle,
            size: 0,
            memory_type_idx: 0,
        };
        let result = device.import(import_info);
        let buffer = return_on_error!(result);
        *buffer_out = Box::into_raw(Box::new(buffer)) as _;
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_get_sync_properties(
    _device: *mut c_void,
    properties: &mut magma_sync_properties,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        properties.capabilities = (MagmaSyncCapability::Binary
            | MagmaSyncCapability::Timeline
            | MagmaSyncCapability::CpuWait
            | MagmaSyncCapability::CpuSignal
            | MagmaSyncCapability::ExportSyncFile
            | MagmaSyncCapability::ImportSyncFile)
            .bits();
        properties.max_timeline_value_difference = u64::MAX;
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

use magma_gpu_magma::MagmaMemoryProperties;

#[allow(non_camel_case_types)]
type magma_memory_properties = MagmaMemoryProperties;

fn vec_into_raw<T>(mut vec: Vec<T>) -> *mut T {
    if vec.is_empty() {
        std::ptr::null_mut()
    } else {
        vec.shrink_to_fit();
        let mut vec = std::mem::ManuallyDrop::new(vec);
        vec.as_mut_ptr()
    }
}

#[no_mangle]
pub unsafe extern "C" fn magma_get_memory_properties(
    physical_device: &mut magma_physical_device,
    properties: &mut magma_memory_properties,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let mem_types = return_on_error!(physical_device.get_memory_types());
        let mem_heaps = return_on_error!(physical_device.get_memory_heaps());

        properties.memory_type_count = mem_types.len() as u32;
        properties.memory_heap_count = mem_heaps.len() as u32;
        properties.memory_types = vec_into_raw(mem_types);
        properties.memory_heaps = vec_into_raw(mem_heaps);

        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}
