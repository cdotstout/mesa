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
use magma_gpu_magma::MagmaAddressSpaceProperties;
use magma_gpu_magma::MagmaBufferFlag;
use magma_gpu_magma::MagmaBufferPlacementInfo;
use magma_gpu_magma::MagmaCreateBufferInfo;
use magma_gpu_magma::MagmaCreateQueueInfo;
use magma_gpu_magma::MagmaCreateSemaphoreInfo;
use magma_gpu_magma::MagmaDeviceCreateInfo;
use magma_gpu_magma::MagmaGpuMapFlags;
use magma_gpu_magma::MagmaHandle;
use magma_gpu_magma::MagmaHandleType;
use magma_gpu_magma::MagmaHeap;
use magma_gpu_magma::MagmaHeapFlag;
use magma_gpu_magma::MagmaMapping;
use magma_gpu_magma::MagmaMemoryHeapsProperties;
use magma_gpu_magma::MagmaPciBusInfo;
use magma_gpu_magma::MagmaPhysicalDeviceInfo;
use magma_gpu_magma::MagmaQueueFamilyProperties;
use magma_gpu_magma::MagmaQueueFlags;
use magma_gpu_magma::MagmaQueuePriority;
use magma_gpu_magma::MagmaSemaphoreType;
use magma_gpu_magma::MagmaSemaphoreUseFlags;
use magma_gpu_magma::MagmaSemaphoreWaitFlags;
use magma_gpu_magma::MagmaStructureType;
use magma_gpu_magma::MagmaStructureTypeHeader;
use magma_gpu_magma::MagmaSubmitAddressSpaceInfo;
use magma_gpu_magma::MagmaSubmitBufferInfo;
use magma_gpu_magma::MagmaSubmitInfo;
use magma_gpu_magma::MagmaSubmitMapBufferInfo;
use magma_gpu_magma::MagmaSubmitResourcesInfo;
use magma_gpu_magma::MagmaSubmitSyncInfo;
use magma_gpu_magma::MagmaSubmitUnmapBufferInfo;
use magma_gpu_magma::MagmaSyncCapability;
use magma_gpu_magma::MagmaSyncProperties;
use magma_gpu_magma::MagmaTimestampProperties;
use magma_gpu_magma::MagmaVendorQuery;
use magma_gpu_magma::PhysicalDevice;
use magma_gpu_magma::Queue;
use magma_gpu_magma::Semaphore;
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

pub const MAGMA_MAX_PHYSICAL_DEVICES: usize = 8;
pub const MAGMA_MAX_MEMORY_HEAPS: usize = 32;
pub const MAGMA_MAX_QUEUES: usize = 16;
#[allow(non_camel_case_types)]
pub type magma_queue = Queue;

#[allow(non_camel_case_types)]
pub type magma_address_space = AddressSpace;

#[allow(non_camel_case_types)]
pub type magma_buffer = Buffer;

#[allow(non_camel_case_types)]
pub type magma_semaphore = Semaphore;

#[allow(non_camel_case_types)]
pub type magma_physical_device = PhysicalDevice;

#[allow(non_camel_case_types)]
pub type magma_device = Device;

#[allow(non_camel_case_types)]
pub type magma_queue_priority = MagmaQueuePriority;

#[allow(non_camel_case_types)]
pub type magma_semaphore_type = MagmaSemaphoreType;

#[allow(non_camel_case_types)]
pub type magma_queue_flags = MagmaQueueFlags;

#[allow(non_camel_case_types)]
pub type magma_gpu_map_flags = MagmaGpuMapFlags;

#[allow(non_camel_case_types)]
pub type magma_buffer_flag = MagmaBufferFlag;

#[allow(non_camel_case_types)]
pub type magma_sync_capability = MagmaSyncCapability;

#[allow(non_camel_case_types)]
pub type magma_semaphore_use_flags = MagmaSemaphoreUseFlags;

#[allow(non_camel_case_types)]
pub type magma_semaphore_wait_flags = MagmaSemaphoreWaitFlags;

#[allow(non_camel_case_types)]
pub type magma_handle_type = MagmaHandleType;

#[allow(non_camel_case_types)]
pub type magma_heap_flag = MagmaHeapFlag;

#[allow(non_camel_case_types)]
pub type magma_heap = MagmaHeap;

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
pub type magma_buffer_placement_info = MagmaBufferPlacementInfo;

#[allow(non_camel_case_types)]
pub type magma_device_create_info = MagmaDeviceCreateInfo;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct magma_submit_sync_info {
    pub header: MagmaStructureTypeHeader,
    pub num_wait_semaphores: u32,
    pub num_signal_semaphores: u32,
    pub wait_semaphores: *mut *mut magma_semaphore,
    pub signal_semaphores: *mut *mut magma_semaphore,
    pub wait_points: *mut u64,
    pub signal_points: *mut u64,
}

impl magma_submit_sync_info {
    pub fn to_rust(&self) -> MagmaSubmitSyncInfo {
        let limit_wait_semaphores = self.num_wait_semaphores as usize;
        let mut wait_semaphores: Vec<Option<Semaphore>> = Vec::with_capacity(limit_wait_semaphores);
        if !self.wait_semaphores.is_null() && limit_wait_semaphores > 0 {
            let slice = unsafe { from_raw_parts(self.wait_semaphores, limit_wait_semaphores) };
            for &ptr in slice {
                let val = if !ptr.is_null() {
                    Some(unsafe { (*ptr).clone() })
                } else {
                    None
                };
                wait_semaphores.push(val);
            }
        } else if limit_wait_semaphores > 0 {
            wait_semaphores.resize(limit_wait_semaphores, None);
        }
        let limit_signal_semaphores = self.num_signal_semaphores as usize;
        let mut signal_semaphores: Vec<Option<Semaphore>> = Vec::with_capacity(limit_signal_semaphores);
        if !self.signal_semaphores.is_null() && limit_signal_semaphores > 0 {
            let slice = unsafe { from_raw_parts(self.signal_semaphores, limit_signal_semaphores) };
            for &ptr in slice {
                let val = if !ptr.is_null() {
                    Some(unsafe { (*ptr).clone() })
                } else {
                    None
                };
                signal_semaphores.push(val);
            }
        } else if limit_signal_semaphores > 0 {
            signal_semaphores.resize(limit_signal_semaphores, None);
        }
        let limit_wait_points = self.num_wait_semaphores as usize;
        let wait_points = if !self.wait_points.is_null() && limit_wait_points > 0 {
            unsafe { from_raw_parts(self.wait_points, limit_wait_points) }.to_vec()
        } else {
            vec![Default::default(); limit_wait_points]
        };
        let limit_signal_points = self.num_signal_semaphores as usize;
        let signal_points = if !self.signal_points.is_null() && limit_signal_points > 0 {
            unsafe { from_raw_parts(self.signal_points, limit_signal_points) }.to_vec()
        } else {
            vec![Default::default(); limit_signal_points]
        };
        MagmaSubmitSyncInfo {
            header: MagmaStructureTypeHeader {
                stype: self.header.stype,
                size: std::mem::size_of::<MagmaSubmitSyncInfo>() as u32,
                p_next: self.header.p_next as usize as u64,
            },
            num_wait_semaphores: self.num_wait_semaphores,
            num_signal_semaphores: self.num_signal_semaphores,
            wait_semaphores,
            signal_semaphores,
            wait_points,
            signal_points,
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
                p_next: self.header.p_next as usize as u64,
            },
            flags: self.flags,
            sync_info: self.sync_info.to_rust(),
            ..Default::default()
        }
    }
}

enum magma_submit_info_ext {
    SubmitAddressSpaceInfo(MagmaSubmitAddressSpaceInfo),
    SubmitBufferInfo(MagmaSubmitBufferInfo),
    SubmitMapBufferInfo(MagmaSubmitMapBufferInfo),
    SubmitUnmapBufferInfo(MagmaSubmitUnmapBufferInfo),
    SubmitResourcesInfo(MagmaSubmitResourcesInfo),
}

impl magma_submit_info_ext {
    fn header_ptr(&self) -> u64 {
        match self {
            Self::SubmitAddressSpaceInfo(ext) => &ext.header as *const _ as u64,
            Self::SubmitBufferInfo(ext) => &ext.header as *const _ as u64,
            Self::SubmitMapBufferInfo(ext) => &ext.header as *const _ as u64,
            Self::SubmitUnmapBufferInfo(ext) => &ext.header as *const _ as u64,
            Self::SubmitResourcesInfo(ext) => &ext.header as *const _ as u64,
        }
    }

    fn set_p_next(&mut self, p_next: u64) {
        match self {
            Self::SubmitAddressSpaceInfo(ext) => ext.header.p_next = p_next,
            Self::SubmitBufferInfo(ext) => ext.header.p_next = p_next,
            Self::SubmitMapBufferInfo(ext) => ext.header.p_next = p_next,
            Self::SubmitUnmapBufferInfo(ext) => ext.header.p_next = p_next,
            Self::SubmitResourcesInfo(ext) => ext.header.p_next = p_next,
        }
    }
}

impl magma_submit_info {
    fn convert_p_next_chain(&self) -> Vec<magma_submit_info_ext> {
        let mut exts = Vec::new();
        let mut curr = self.header.p_next as usize;
        while curr != 0 {
            let header = unsafe { &*(curr as *const MagmaStructureTypeHeader) };
            let next = header.p_next as usize;
            if header.stype == MagmaStructureType::SubmitAddressSpaceInfo as u32 {
                let mut ext = unsafe { *(curr as *const magma_submit_address_space_info) };
                ext.header.p_next = 0;
                exts.push(magma_submit_info_ext::SubmitAddressSpaceInfo(ext));
            } else if header.stype == MagmaStructureType::SubmitBufferInfo as u32 {
                let mut ext = unsafe { (*(curr as *const magma_submit_buffer_info)).to_rust() };
                ext.header.p_next = 0;
                exts.push(magma_submit_info_ext::SubmitBufferInfo(ext));
            } else if header.stype == MagmaStructureType::SubmitMapBufferInfo as u32 {
                let mut ext = unsafe { (*(curr as *const magma_submit_map_buffer_info)).to_rust() };
                ext.header.p_next = 0;
                exts.push(magma_submit_info_ext::SubmitMapBufferInfo(ext));
            } else if header.stype == MagmaStructureType::SubmitUnmapBufferInfo as u32 {
                let mut ext = unsafe { *(curr as *const magma_submit_unmap_buffer_info) };
                ext.header.p_next = 0;
                exts.push(magma_submit_info_ext::SubmitUnmapBufferInfo(ext));
            } else if header.stype == MagmaStructureType::SubmitResourcesInfo as u32 {
                let mut ext = unsafe { (*(curr as *const magma_submit_resources_info)).to_rust() };
                ext.header.p_next = 0;
                exts.push(magma_submit_info_ext::SubmitResourcesInfo(ext));
            }
            curr = next;
        }
        for i in (0..exts.len()).rev() {
            let next_ptr = if i + 1 < exts.len() {
                exts[i + 1].header_ptr()
            } else {
                0
            };
            exts[i].set_p_next(next_ptr);
        }
        exts
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
        let command_buffer = if !self.command_buffer.is_null() {
            self.command_buffer
        } else {
            null_mut()
        };
        MagmaSubmitBufferInfo {
            header: MagmaStructureTypeHeader {
                stype: self.header.stype,
                size: std::mem::size_of::<MagmaSubmitBufferInfo>() as u32,
                p_next: self.header.p_next as usize as u64,
            },
            command_buffer,
            start_offset: self.start_offset,
            length: self.length,
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct magma_submit_map_buffer_info {
    pub header: MagmaStructureTypeHeader,
    pub buffer: *mut magma_buffer,
    pub buffer_offset: u64,
    pub gpu_va: u64,
    pub size: u64,
    pub flags: MagmaGpuMapFlags,
}

impl magma_submit_map_buffer_info {
    pub fn to_rust(&self) -> MagmaSubmitMapBufferInfo {
        let buffer = if !self.buffer.is_null() {
            self.buffer
        } else {
            null_mut()
        };
        MagmaSubmitMapBufferInfo {
            header: MagmaStructureTypeHeader {
                stype: self.header.stype,
                size: std::mem::size_of::<MagmaSubmitMapBufferInfo>() as u32,
                p_next: self.header.p_next as usize as u64,
            },
            buffer,
            buffer_offset: self.buffer_offset,
            gpu_va: self.gpu_va,
            size: self.size,
            flags: self.flags,
            ..Default::default()
        }
    }
}

#[allow(non_camel_case_types)]
pub type magma_submit_unmap_buffer_info = MagmaSubmitUnmapBufferInfo;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct magma_submit_resources_info {
    pub header: MagmaStructureTypeHeader,
    pub num_resources: u32,
    pub buffers: *mut *mut magma_buffer,
}

impl magma_submit_resources_info {
    pub fn to_rust(&self) -> MagmaSubmitResourcesInfo {
        let limit_buffers = self.num_resources as usize;
        let mut buffers: Vec<*mut Buffer> = Vec::with_capacity(limit_buffers);
        if !self.buffers.is_null() && limit_buffers > 0 {
            let slice = unsafe { from_raw_parts(self.buffers, limit_buffers) };
            for &ptr in slice {
                let val = if !ptr.is_null() {
                    ptr
                } else {
                    null_mut()
                };
                buffers.push(val);
            }
        } else if limit_buffers > 0 {
            buffers.resize(limit_buffers, null_mut());
        }
        MagmaSubmitResourcesInfo {
            header: MagmaStructureTypeHeader {
                stype: self.header.stype,
                size: std::mem::size_of::<MagmaSubmitResourcesInfo>() as u32,
                p_next: self.header.p_next as usize as u64,
            },
            num_resources: self.num_resources,
            buffers,
            ..Default::default()
        }
    }
}

#[allow(non_camel_case_types)]
pub type magma_create_semaphore_info = MagmaCreateSemaphoreInfo;

#[allow(non_camel_case_types)]
pub type magma_sync_properties = MagmaSyncProperties;

#[allow(non_camel_case_types)]
pub type magma_physical_device_info = MagmaPhysicalDeviceInfo;

#[allow(non_camel_case_types)]
pub type magma_address_space_properties = MagmaAddressSpaceProperties;

#[allow(non_camel_case_types)]
pub type magma_timestamp_properties = MagmaTimestampProperties;

#[allow(non_camel_case_types)]
pub type magma_vendor_query = MagmaVendorQuery;

#[allow(non_camel_case_types)]
pub type magma_memory_heaps_properties = MagmaMemoryHeapsProperties;

#[no_mangle]
pub unsafe extern "C" fn magma_physical_device_create_device(
    physical_device: &mut magma_physical_device,
    device: &mut *mut magma_device,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_physical_device_create_device");
        let result = physical_device.create_device();
        let res = return_on_error!(result);
        *device = Box::into_raw(Box::new(res)) as _;
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_device_create_buffer(
    device: &mut magma_device,
    info: &magma_create_buffer_info,
    buffer_out: &mut *mut magma_buffer,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_device_create_buffer");
        let result = device.create_buffer(info);
        let res = return_on_error!(result);
        *buffer_out = Box::into_raw(Box::new(res)) as _;
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_device_create_address_space(
    device: &mut magma_device,
    address_space: &mut *mut magma_address_space,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_device_create_address_space");
        let result = device.create_address_space();
        let res = return_on_error!(result);
        *address_space = Box::into_raw(Box::new(res)) as _;
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_device_create_queue(
    device: &mut magma_device,
    address_space: &mut magma_address_space,
    info: &magma_create_queue_info,
    queue: &mut *mut magma_queue,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_device_create_queue");
        let result = device.create_queue(address_space, info);
        let res = return_on_error!(result);
        *queue = Box::into_raw(Box::new(res)) as _;
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_device_release(device: &mut *mut magma_device) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_device_release");
        if !(*device).is_null() {
            let _ = unsafe { Box::from_raw(*device) };
            *device = null_mut();
        }
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_physical_device_release(
    physical_device: &mut *mut magma_physical_device,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_physical_device_release");
        if !(*physical_device).is_null() {
            let _ = unsafe { Box::from_raw(*physical_device) };
            *physical_device = null_mut();
        }
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_buffer_release(buffer: &mut *mut magma_buffer) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_buffer_release");
        if !(*buffer).is_null() {
            let _ = unsafe { Box::from_raw(*buffer) };
            *buffer = null_mut();
        }
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_queue_release(queue: &mut *mut magma_queue) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_queue_release");
        if !(*queue).is_null() {
            let _ = unsafe { Box::from_raw(*queue) };
            *queue = null_mut();
        }
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_address_space_release(
    address_space: &mut *mut magma_address_space,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_address_space_release");
        if !(*address_space).is_null() {
            let _ = unsafe { Box::from_raw(*address_space) };
            *address_space = null_mut();
        }
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_address_space_map_buffer(
    address_space: &mut magma_address_space,
    buffer: &mut magma_buffer,
    buffer_offset: u64,
    gpu_va: u64,
    size: u64,
    flags: magma_gpu_map_flags,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_address_space_map_buffer");
        let result = address_space.map_buffer_gpu(buffer, buffer_offset, gpu_va, size, flags);
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_address_space_unmap_buffer(
    address_space: &mut magma_address_space,
    gpu_va: u64,
    size: u64,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_address_space_unmap_buffer");
        let result = address_space.unmap_buffer_gpu(gpu_va, size);
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_queue_submit(
    queue: &mut magma_queue,
    submit_info: &magma_submit_info,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_queue_submit");
        let rust_submit_info_exts = submit_info.convert_p_next_chain();
        let mut rust_submit_info = submit_info.to_rust();
        if rust_submit_info.header.p_next != 0 {
            rust_submit_info.header.p_next =
                rust_submit_info_exts.first().map_or(0, |ext| ext.header_ptr());
        }
        let result = queue.submit_command(&rust_submit_info);
        let _ = &rust_submit_info_exts;
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_device_create_semaphore(
    device: &mut magma_device,
    info: &magma_create_semaphore_info,
    semaphore: &mut *mut magma_semaphore,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_device_create_semaphore");
        let result = device.create_semaphore(info);
        let res = return_on_error!(result);
        *semaphore = Box::into_raw(Box::new(res)) as _;
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_semaphore_release(semaphore: &mut *mut magma_semaphore) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_semaphore_release");
        if !(*semaphore).is_null() {
            let _ = unsafe { Box::from_raw(*semaphore) };
            *semaphore = null_mut();
        }
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_semaphore_wait(
    semaphore: &mut magma_semaphore,
    abs_timeout_ns: u64,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_semaphore_wait");
        let result = semaphore.wait(abs_timeout_ns);
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_semaphore_signal(semaphore: &mut magma_semaphore) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_semaphore_signal");
        let result = semaphore.signal();
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_semaphore_reset(semaphore: &mut magma_semaphore) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_semaphore_reset");
        let result = semaphore.reset();
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_semaphore_timeline_wait(
    semaphore: &mut magma_semaphore,
    point: u64,
    abs_timeout_ns: u64,
    flags: u32,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_semaphore_timeline_wait");
        let result = semaphore.timeline_wait(point, abs_timeout_ns, flags);
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_semaphore_timeline_signal(
    semaphore: &mut magma_semaphore,
    point: u64,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_semaphore_timeline_signal");
        let result = semaphore.timeline_signal(point);
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_semaphore_timeline_query(
    semaphore: &mut magma_semaphore,
    point_out: &mut u64,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_semaphore_timeline_query");
        let result = semaphore.timeline_query();
        *point_out = return_on_error!(result);
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
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
pub unsafe extern "C" fn magma_buffer_map(
    buffer: &mut magma_buffer,
    mapping_out: &mut magma_mapping,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_buffer_map");
        let result = buffer.map_cpu();
        *mapping_out = return_on_error!(result);
        NO_ERROR
    }))
    .unwrap_or(-ESRCH)
}

#[no_mangle]
pub unsafe extern "C" fn magma_buffer_unmap(buffer: &mut magma_buffer) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        debug!("call: magma_buffer_unmap");
        let result = buffer.unmap_cpu();
        return_result(result)
    }))
    .unwrap_or(-ESRCH)
}
