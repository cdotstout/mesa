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
use magma_gpu::util::RawDescriptor;
use magma_gpu_magma::Buffer;
use magma_gpu_magma::Device;
use magma_gpu_magma::MagmaAddressSpaceProperties;
use magma_gpu_magma::MagmaHandle;
use magma_gpu_magma::MagmaImportHandleInfo;
use magma_gpu_magma::MagmaMemoryProperties;
use magma_gpu_magma::MagmaPhysicalDeviceInfo;
use magma_gpu_magma::MagmaStructureTypeHeader;
use magma_gpu_magma::MagmaSyncCapability;
use magma_gpu_magma::MagmaSyncProperties;
use magma_gpu_magma::MagmaTimestampProperties;
use magma_gpu_magma::MagmaVendorQuery;
use magma_gpu_magma::MagmaVendorQueryResult;
use magma_gpu_magma::Semaphore;
use magma_gpu_magma::MAGMA_MAX_MEMORY_HEAPS;
use magma_gpu_magma::MAGMA_MAX_MEMORY_TYPES;
use magma_gpu_magma::MAGMA_STRUCTURE_TYPE_ADDRESS_SPACE_PROPERTIES;
use magma_gpu_magma::MAGMA_STRUCTURE_TYPE_MEMORY_PROPERTIES;
use magma_gpu_magma::MAGMA_STRUCTURE_TYPE_PHYSICAL_DEVICE_INFO;
use magma_gpu_magma::MAGMA_STRUCTURE_TYPE_SYNC_PROPERTIES;
use magma_gpu_magma::MAGMA_STRUCTURE_TYPE_TIMESTAMP_PROPERTIES;
use magma_gpu_magma::MAGMA_STRUCTURE_TYPE_VENDOR_QUERY;
use std::ffi::c_void;

#[allow(non_camel_case_types)]
type magma_device = Device;

#[allow(non_camel_case_types)]
type magma_buffer = Buffer;

#[allow(non_camel_case_types)]
type magma_semaphore = Semaphore;

#[allow(non_camel_case_types)]
type magma_handle = MagmaHandle;

#[no_mangle]
pub unsafe extern "C" fn magma_semaphore_export(
    semaphore: &mut magma_semaphore,
    handle_out: &mut magma_handle,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let result = semaphore.export_fence();
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
pub unsafe extern "C" fn magma_semaphore_import(
    semaphore: &mut magma_semaphore,
    handle: &magma_handle,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let descriptor = OwnedDescriptor::from_raw_descriptor(handle.os_handle as RawDescriptor);
        let gpu_handle = MagmaGpuHandle {
            os_handle: descriptor,
            handle_type: handle.handle_type,
        };
        let result = semaphore.import(gpu_handle);
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
pub unsafe extern "C" fn magma_device_import_buffer(
    device: &mut magma_device,
    handle: &magma_handle,
    buffer_out: &mut *mut magma_buffer,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let descriptor = OwnedDescriptor::from_raw_descriptor(handle.os_handle as RawDescriptor);
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
pub unsafe extern "C" fn magma_physical_device_query(
    physical_device: &mut magma_physical_device,
    query: *mut c_void,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if query.is_null() {
            return -EINVAL;
        }

        let mut curr = query as usize;
        while curr != 0 {
            let header = &*(curr as *const MagmaStructureTypeHeader);
            let next = header.p_next as usize;
            match header.stype {
                MAGMA_STRUCTURE_TYPE_PHYSICAL_DEVICE_INFO => {
                    let out = &mut *(curr as *mut MagmaPhysicalDeviceInfo);
                    *out = *physical_device.info();
                    out.header.stype = MAGMA_STRUCTURE_TYPE_PHYSICAL_DEVICE_INFO;
                    out.header.size = std::mem::size_of::<MagmaPhysicalDeviceInfo>() as u32;
                    out.header.p_next = next as u64;
                }
                MAGMA_STRUCTURE_TYPE_MEMORY_PROPERTIES => {
                    let out = &mut *(curr as *mut MagmaMemoryProperties);
                    let mem_types = return_on_error!(physical_device.get_memory_types());
                    let mem_heaps = return_on_error!(physical_device.get_memory_heaps());
                    let type_count = mem_types.len().min(MAGMA_MAX_MEMORY_TYPES);
                    let heap_count = mem_heaps.len().min(MAGMA_MAX_MEMORY_HEAPS);
                    out.memory_type_count = type_count as u32;
                    out.memory_heap_count = heap_count as u32;
                    out.memory_types = [Default::default(); MAGMA_MAX_MEMORY_TYPES];
                    out.memory_types[..type_count].copy_from_slice(&mem_types[..type_count]);
                    out.memory_heaps = [Default::default(); MAGMA_MAX_MEMORY_HEAPS];
                    out.memory_heaps[..heap_count].copy_from_slice(&mem_heaps[..heap_count]);
                }
                MAGMA_STRUCTURE_TYPE_SYNC_PROPERTIES => {
                    let out = &mut *(curr as *mut MagmaSyncProperties);
                    out.capabilities = (MagmaSyncCapability::Binary
                        | MagmaSyncCapability::Timeline
                        | MagmaSyncCapability::CpuWait
                        | MagmaSyncCapability::CpuSignal
                        | MagmaSyncCapability::ExportSyncFile
                        | MagmaSyncCapability::ImportSyncFile)
                        .bits();
                    out.max_timeline_value_difference = u64::MAX;
                }
                MAGMA_STRUCTURE_TYPE_ADDRESS_SPACE_PROPERTIES => {
                    let out = &mut *(curr as *mut MagmaAddressSpaceProperties);
                    *out = return_on_error!(physical_device.get_address_space_properties());
                    out.header.p_next = next as u64;
                }
                MAGMA_STRUCTURE_TYPE_TIMESTAMP_PROPERTIES => {
                    let out = &mut *(curr as *mut MagmaTimestampProperties);
                    *out = return_on_error!(physical_device.get_timestamp_properties());
                    out.header.p_next = next as u64;
                }
                MAGMA_STRUCTURE_TYPE_VENDOR_QUERY => {
                    let out = &mut *(curr as *mut MagmaVendorQuery);
                    let res = if out.buffer.is_null() {
                        return_on_error!(physical_device.query_vendor(out.query_id, None))
                    } else {
                        let buf = from_raw_parts_mut(
                            out.buffer as *mut u8,
                            out.buffer_size as usize,
                        );
                        return_on_error!(physical_device.query_vendor(out.query_id, Some(buf)))
                    };
                    match res {
                        MagmaVendorQueryResult::Simple(val) => {
                            out.result = val;
                            out.buffer_size = 0;
                        }
                        MagmaVendorQueryResult::Buffer(len) => {
                            out.buffer_size = len as u64;
                        }
                    }
                    out.header.p_next = next as u64;
                }
                _ => return -EINVAL,
            }
            curr = next;
        }

        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_init_open_in_namespace_callback(callback: *mut c_void) {
    #[cfg(target_os = "fuchsia")]
    {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            magma_gpu_magma::init_open_in_namespace_callback(callback);
        }));
    }
    #[cfg(not(target_os = "fuchsia"))]
    {
        let _ = callback;
    }
}
