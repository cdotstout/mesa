// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::null_mut;
use std::sync::Arc;

use magma_gpu::util::Handle as MagmaGpuHandle;
use magma_gpu::util::MAGMA_MAP_ACCESS_RW;
use magma_gpu::util::MAGMA_MAP_CACHE_CACHED;
use magma_gpu::util::MAGMA_MAP_CACHE_WC;
use zerocopy::FromBytes;
use zerocopy::IntoBytes;

pub use crate::protocol::*;
use crate::memory::Buffer;
use crate::semaphore::Semaphore;
use crate::traits::BackendSemaphore;

pub const MAGMA_BUS_TYPE_UNKNOWN: u32 = 0;
pub const MAGMA_BUS_TYPE_PCI: u32 = 1;
pub const MAGMA_BUS_TYPE_PLATFORM: u32 = 2;

pub const MAGMA_STRUCTURE_TYPE_VENDOR_QUERY: u32 = 0x00000013;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct MagmaVendorQuery {
    pub header: MagmaStructureTypeHeader,
    pub query_id: u64,
    pub result: u64,
    pub buffer: *mut c_void,
    pub buffer_size: u64,
}

// Same as PCI id
pub const MAGMA_VENDOR_ID_INTEL: u16 = 0x8086;
pub const MAGMA_VENDOR_ID_AMD: u16 = 0x1002;
pub const MAGMA_VENDOR_ID_MALI: u16 = 0x13B5;
pub const MAGMA_VENDOR_ID_QCOM: u16 = 0x5413;
pub const MAGMA_VENDOR_ID_VIRTGPU: u16 = 0x1AF4;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagmaMapping {
    pub ptr: *mut c_void,
    pub size: u64,
}

unsafe impl Send for MagmaMapping {}
unsafe impl Sync for MagmaMapping {}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MagmaHandle {
    pub os_handle: i64,
    pub handle_type: u32,
}

// Common allocation flags
//  - MAGMA_BUFFER_FLAG_EXTERNAL: The buffer *may* be exported as an OS-specific handle
//  - MAGMA_BUFFER_FLAG_SCANOUT: The buffer *may* be used by the scanout engine directly
//  - MAGMA_BUFFER_FLAG_CACHED: Request CPU write-back caching (otherwise write-combining)
//  - MAGMA_BUFFER_FLAG_PROTECTED: Request protected/encrypted memory
pub const MAGMA_BUFFER_FLAG_EXTERNAL: u32 = 0x000000001;
pub const MAGMA_BUFFER_FLAG_SCANOUT: u32 = 0x000000002;
pub const MAGMA_BUFFER_FLAG_CACHED: u32 = 0x000000004;
pub const MAGMA_BUFFER_FLAG_PROTECTED: u32 = 0x000000008;

pub fn buffer_flags_get_map_info(common_flags: u32) -> Option<u32> {
    let cache = if (common_flags & MAGMA_BUFFER_FLAG_CACHED) != 0
        && (common_flags & MAGMA_BUFFER_FLAG_SCANOUT) == 0
    {
        MAGMA_MAP_CACHE_CACHED
    } else {
        MAGMA_MAP_CACHE_WC
    };
    Some(cache | MAGMA_MAP_ACCESS_RW)
}

impl MagmaCreateBufferInfo {
    pub fn is_scanout(&self) -> bool {
        (self.common_flags & MAGMA_BUFFER_FLAG_SCANOUT) != 0
    }

    pub fn is_cached(&self) -> bool {
        (self.common_flags & MAGMA_BUFFER_FLAG_CACHED) != 0
    }

    pub fn is_protected(&self) -> bool {
        (self.common_flags & MAGMA_BUFFER_FLAG_PROTECTED) != 0
    }

    pub fn get_map_info(&self) -> Option<u32> {
        buffer_flags_get_map_info(self.common_flags)
    }
}

// Acceptable buffer vendor flags if the vendor is AMD:
//  - MAGMA_BUFFER_FLAG_AMD_FLAG_OA: Ordered append, used by 3D/Compute engines
//  - MAGMA_BUFFER_FLAG_AMD_FLAG_GDS: Global on-chip data storage. Used to share
//                                    data across shader threads
pub const MAGMA_BUFFER_FLAG_AMD_OA: u32 = 0x000000001;
pub const MAGMA_BUFFER_FLAG_AMD_GDS: u32 = 0x000000002;

pub const MAGMA_SYNC_WHOLE_RANGE: u64 = 1 << 0;
pub const MAGMA_SYNC_RANGES: u64 = 1 << 1;
pub const MAGMA_SYNC_INVALIDATE_READ: u64 = 1 << 2;
pub const MAGMA_SYNC_INVALIDATE_WRITE: u64 = 1 << 3;

pub type MagmaSyncType = MagmaSemaphoreType;
pub const MAGMA_SEMAPHORE_TYPE_BINARY: u32 = MagmaSemaphoreType::Binary as u32;
pub const MAGMA_SEMAPHORE_TYPE_TIMELINE: u32 = MagmaSemaphoreType::Timeline as u32;

impl From<u32> for MagmaSyncType {
    fn from(val: u32) -> MagmaSyncType {
        match val {
            1 => MagmaSyncType::Timeline,
            _ => MagmaSyncType::Binary,
        }
    }
}

#[repr(C)]
#[derive(Clone, Default, Debug, IntoBytes, FromBytes)]
pub struct MagmaMappedMemoryRange {
    pub offset: u64,
    pub size: u64,
}

pub struct MagmaImportHandleInfo {
    pub handle: MagmaGpuHandle,
    pub size: u64,
    pub common_flags: u32,
}

#[derive(Clone, Default)]
pub struct MagmaSubmitSyncInfo {
    pub header: MagmaStructureTypeHeader,
    pub num_wait_semaphores: u32,
    pub num_signal_semaphores: u32,
    pub wait_semaphores: Vec<Option<Semaphore>>,
    pub signal_semaphores: Vec<Option<Semaphore>>,
    pub wait_points: Vec<u64>,
    pub signal_points: Vec<u64>,
}

impl MagmaSubmitSyncInfo {
    pub fn from_semaphores(
        wait_semaphores: &[Semaphore],
        signal_semaphores: &[Semaphore],
    ) -> MagmaSubmitSyncInfo {
        let num_wait = wait_semaphores.len();
        let num_signal = signal_semaphores.len();
        let wait = wait_semaphores.iter().cloned().map(Some).collect();
        let signal = signal_semaphores.iter().cloned().map(Some).collect();
        MagmaSubmitSyncInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitSyncInfo as u32,
                size: size_of::<MagmaSubmitSyncInfo>() as u32,
                p_next: 0,
            },
            num_wait_semaphores: num_wait as u32,
            num_signal_semaphores: num_signal as u32,
            wait_semaphores: wait,
            signal_semaphores: signal,
            wait_points: vec![0; num_wait],
            signal_points: vec![0; num_signal],
        }
    }

    pub fn wait_semaphores(&self) -> impl Iterator<Item = &Arc<dyn BackendSemaphore>> {
        self.wait_semaphores
            .iter()
            .take(self.num_wait_semaphores as usize)
            .filter_map(|opt| opt.as_ref().map(|obj| obj.semaphore()))
    }

    pub fn wait_semaphores_with_points(
        &self,
    ) -> impl Iterator<Item = (&Arc<dyn BackendSemaphore>, u64)> {
        self.wait_semaphores
            .iter()
            .zip(self.wait_points.iter().copied().chain(std::iter::repeat(0)))
            .take(self.num_wait_semaphores as usize)
            .filter_map(|(opt, point)| opt.as_ref().map(|obj| (obj.semaphore(), point)))
    }

    pub fn signal_semaphores(&self) -> impl Iterator<Item = &Arc<dyn BackendSemaphore>> {
        self.signal_semaphores
            .iter()
            .take(self.num_signal_semaphores as usize)
            .filter_map(|opt| opt.as_ref().map(|obj| obj.semaphore()))
    }

    pub fn signal_semaphores_with_points(
        &self,
    ) -> impl Iterator<Item = (&Arc<dyn BackendSemaphore>, u64)> {
        self.signal_semaphores
            .iter()
            .zip(self.signal_points.iter().copied().chain(std::iter::repeat(0)))
            .take(self.num_signal_semaphores as usize)
            .filter_map(|(opt, point)| opt.as_ref().map(|obj| (obj.semaphore(), point)))
    }
}

#[derive(Clone, Default)]
pub struct MagmaSubmitInfo {
    pub header: MagmaStructureTypeHeader,
    pub flags: u32,
    pub sync_info: MagmaSubmitSyncInfo,
}

impl MagmaSubmitInfo {
    pub fn new_with_address_space(
        flags: u32,
        sync_info: MagmaSubmitSyncInfo,
        as_info: &MagmaSubmitAddressSpaceInfo,
    ) -> MagmaSubmitInfo {
        MagmaSubmitInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitInfo as u32,
                size: size_of::<MagmaSubmitInfo>() as u32,
                p_next: as_info as *const _ as u64,
            },
            flags,
            sync_info,
        }
    }

    pub fn new_with_buffer(
        flags: u32,
        sync_info: MagmaSubmitSyncInfo,
        buf_info: &MagmaSubmitBufferInfo,
    ) -> MagmaSubmitInfo {
        MagmaSubmitInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitInfo as u32,
                size: size_of::<MagmaSubmitInfo>() as u32,
                p_next: buf_info as *const _ as u64,
            },
            flags,
            sync_info,
        }
    }

    pub fn new_with_map_buffer(
        flags: u32,
        sync_info: MagmaSubmitSyncInfo,
        map_info: &MagmaSubmitMapBufferInfo,
    ) -> MagmaSubmitInfo {
        MagmaSubmitInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitInfo as u32,
                size: size_of::<MagmaSubmitInfo>() as u32,
                p_next: map_info as *const _ as u64,
            },
            flags,
            sync_info,
        }
    }

    pub fn new_with_unmap_buffer(
        flags: u32,
        sync_info: MagmaSubmitSyncInfo,
        unmap_info: &MagmaSubmitUnmapBufferInfo,
    ) -> MagmaSubmitInfo {
        MagmaSubmitInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitInfo as u32,
                size: size_of::<MagmaSubmitInfo>() as u32,
                p_next: unmap_info as *const _ as u64,
            },
            flags,
            sync_info,
        }
    }

    pub fn new_with_resources(
        flags: u32,
        sync_info: MagmaSubmitSyncInfo,
        resources_info: &MagmaSubmitResourcesInfo,
    ) -> MagmaSubmitInfo {
        MagmaSubmitInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitInfo as u32,
                size: size_of::<MagmaSubmitInfo>() as u32,
                p_next: resources_info as *const _ as u64,
            },
            flags,
            sync_info,
        }
    }

    pub fn sync_info(&self) -> Option<&MagmaSubmitSyncInfo> {
        if self.sync_info.header.stype == MagmaStructureType::SubmitSyncInfo as u32 {
            Some(&self.sync_info)
        } else {
            None
        }
    }

    pub fn set_sync_info(&mut self, sync_info: MagmaSubmitSyncInfo) {
        self.sync_info = sync_info;
    }

    pub fn payloads(&self) -> MagmaSubmitPayloadIter<'_> {
        MagmaSubmitPayloadIter {
            curr: self.header.p_next as usize,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn address_space_info(&self) -> Option<&MagmaSubmitAddressSpaceInfo> {
        self.payloads().find_map(|p| match p {
            MagmaSubmitPayload::AddressSpace(info) => Some(info),
            _ => None,
        })
    }

    pub fn buffer_info(&self) -> Option<&MagmaSubmitBufferInfo> {
        self.payloads().find_map(|p| match p {
            MagmaSubmitPayload::Buffer(info) => Some(info),
            _ => None,
        })
    }

    pub fn map_buffer_info(&self) -> Option<&MagmaSubmitMapBufferInfo> {
        self.payloads().find_map(|p| match p {
            MagmaSubmitPayload::MapBuffer(info) => Some(info),
            _ => None,
        })
    }

    pub fn unmap_buffer_info(&self) -> Option<&MagmaSubmitUnmapBufferInfo> {
        self.payloads().find_map(|p| match p {
            MagmaSubmitPayload::UnmapBuffer(info) => Some(info),
            _ => None,
        })
    }

    pub fn resources_info(&self) -> Option<&MagmaSubmitResourcesInfo> {
        self.payloads().find_map(|p| match p {
            MagmaSubmitPayload::Resources(info) => Some(info),
            _ => None,
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MagmaSubmitBufferInfo {
    pub header: MagmaStructureTypeHeader,
    pub command_buffer: *mut Buffer,
    pub start_offset: u64,
    pub length: u64,
}

impl Default for MagmaSubmitBufferInfo {
    fn default() -> Self {
        Self {
            header: MagmaStructureTypeHeader::default(),
            command_buffer: null_mut(),
            start_offset: 0,
            length: 0,
        }
    }
}

impl MagmaSubmitBufferInfo {
    pub fn buffer(&self) -> Option<&Buffer> {
        unsafe { self.command_buffer.as_ref() }
    }

    pub fn command_buffer_handle(&self) -> Option<u32> {
        self.buffer().and_then(|b| b.inner().id())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MagmaSubmitMapBufferInfo {
    pub header: MagmaStructureTypeHeader,
    pub buffer: *mut Buffer,
    pub buffer_offset: u64,
    pub gpu_va: u64,
    pub size: u64,
    pub flags: MagmaGpuMapFlags,
}

impl Default for MagmaSubmitMapBufferInfo {
    fn default() -> Self {
        Self {
            header: MagmaStructureTypeHeader::default(),
            buffer: null_mut(),
            buffer_offset: 0,
            gpu_va: 0,
            size: 0,
            flags: MagmaGpuMapFlags::default(),
        }
    }
}

impl MagmaSubmitMapBufferInfo {
    pub fn buffer(&self) -> Option<&Buffer> {
        unsafe { self.buffer.as_ref() }
    }

    pub fn buffer_handle(&self) -> Option<u32> {
        self.buffer().and_then(|b| b.inner().id())
    }

    pub fn buffer_pat_index(&self) -> u16 {
        self.buffer().map_or(0, |b| b.inner().pat_index())
    }
}

#[repr(C)]
#[derive(Clone, Default, Debug)]
pub struct MagmaSubmitResourcesInfo {
    pub header: MagmaStructureTypeHeader,
    pub num_resources: u32,
    pub buffers: Vec<*mut Buffer>,
}

impl MagmaSubmitResourcesInfo {
    pub fn buffers(&self) -> impl Iterator<Item = Option<&Buffer>> {
        self.buffers
            .iter()
            .take(self.num_resources as usize)
            .map(|&ptr| unsafe { ptr.as_ref() })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MagmaSubmitPayload<'a> {
    AddressSpace(&'a MagmaSubmitAddressSpaceInfo),
    Buffer(&'a MagmaSubmitBufferInfo),
    MapBuffer(&'a MagmaSubmitMapBufferInfo),
    UnmapBuffer(&'a MagmaSubmitUnmapBufferInfo),
    Resources(&'a MagmaSubmitResourcesInfo),
}

#[derive(Clone, Copy, Debug)]
pub struct MagmaSubmitPayloadIter<'a> {
    curr: usize,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Iterator for MagmaSubmitPayloadIter<'a> {
    type Item = MagmaSubmitPayload<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.curr != 0 {
            let ptr = self.curr;
            let header = unsafe { &*(ptr as *const MagmaStructureTypeHeader) };
            self.curr = header.p_next as usize;
            match header.stype {
                MAGMA_STRUCTURE_TYPE_SUBMIT_ADDRESS_SPACE_INFO => {
                    return Some(MagmaSubmitPayload::AddressSpace(unsafe {
                        &*(ptr as *const MagmaSubmitAddressSpaceInfo)
                    }));
                }
                MAGMA_STRUCTURE_TYPE_SUBMIT_BUFFER_INFO => {
                    return Some(MagmaSubmitPayload::Buffer(unsafe {
                        &*(ptr as *const MagmaSubmitBufferInfo)
                    }));
                }
                MAGMA_STRUCTURE_TYPE_SUBMIT_MAP_BUFFER_INFO => {
                    return Some(MagmaSubmitPayload::MapBuffer(unsafe {
                        &*(ptr as *const MagmaSubmitMapBufferInfo)
                    }));
                }
                MAGMA_STRUCTURE_TYPE_SUBMIT_UNMAP_BUFFER_INFO => {
                    return Some(MagmaSubmitPayload::UnmapBuffer(unsafe {
                        &*(ptr as *const MagmaSubmitUnmapBufferInfo)
                    }));
                }
                MAGMA_STRUCTURE_TYPE_SUBMIT_RESOURCES_INFO => {
                    return Some(MagmaSubmitPayload::Resources(unsafe {
                        &*(ptr as *const MagmaSubmitResourcesInfo)
                    }));
                }
                _ => continue,
            }
        }
        None
    }
}
