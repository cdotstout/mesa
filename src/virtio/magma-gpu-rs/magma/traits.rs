// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::sync::Arc;
use std::sync::Mutex;

use magma_gpu::util::Handle as MagmaGpuHandle;
use magma_gpu::util::MappedRegion;
use magma_gpu::virtgpu_kumquat::VirtGpuKumquat;

use crate::defines::MagmaAddressSpaceProperties;
use crate::defines::MagmaCreateBufferInfo;
use crate::defines::MagmaCreateQueueInfo;
use crate::defines::MagmaCreateSemaphoreInfo;
use crate::defines::MagmaImportHandleInfo;
use crate::defines::MagmaMappedMemoryRange;
use crate::defines::MagmaPhysicalDeviceInfo;
use crate::defines::MagmaQueueFamilyProperties;
use crate::defines::MagmaSubmitInfo;
use crate::defines::MagmaSyncCapability;
use crate::defines::MagmaSyncProperties;
use crate::defines::MagmaSyncType;
use crate::defines::MagmaTimestampProperties;
use crate::error::Error;
use crate::error::Result;
use crate::protocol::MagmaGpuMapFlags;
use crate::protocol::MagmaSemaphoreUseFlags;
use crate::sys::platform::PlatformDevice;
use crate::sys::platform::PlatformPhysicalDevice;

pub trait AsVirtGpu {
    #[allow(dead_code)]
    fn as_virtgpu(&self) -> Option<&Mutex<VirtGpuKumquat>> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagmaVendorQueryResult {
    Simple(u64),
    Buffer(usize),
}

pub trait GenericPhysicalDevice {
    fn create_device(
        self: Arc<Self>,
        device_info: &MagmaPhysicalDeviceInfo,
    ) -> Result<Arc<dyn BackendDevice>>;

    fn get_device_info(&self, _info: &mut MagmaPhysicalDeviceInfo) -> Result<()> {
        Ok(())
    }

    fn get_queue_family_properties(&self) -> Result<Vec<MagmaQueueFamilyProperties>>;

    fn get_address_space_properties(&self) -> Result<MagmaAddressSpaceProperties> {
        Ok(MagmaAddressSpaceProperties::new(0, 1u64 << 48, 4096))
    }

    fn get_timestamp_properties(&self) -> Result<MagmaTimestampProperties> {
        Ok(MagmaTimestampProperties::new(0))
    }

    fn get_sync_properties(&self) -> Result<MagmaSyncProperties> {
        Ok(MagmaSyncProperties::new(
            (MagmaSyncCapability::Binary
                | MagmaSyncCapability::Timeline
                | MagmaSyncCapability::CpuWait
                | MagmaSyncCapability::CpuSignal
                | MagmaSyncCapability::ExportSyncFile
                | MagmaSyncCapability::ImportSyncFile)
                .bits(),
            u64::MAX,
        ))
    }

    fn query_vendor(
        &self,
        _query_id: u64,
        _buffer: Option<&mut [u8]>,
    ) -> Result<MagmaVendorQueryResult> {
        Err(Error::Unimplemented)
    }
}

pub trait GenericDevice {
    fn create_address_space(self: Arc<Self>) -> Result<Arc<dyn BackendAddressSpace>>;

    fn create_queue(
        self: Arc<Self>,
        address_space: &Arc<dyn BackendAddressSpace>,
        info: &MagmaCreateQueueInfo,
    ) -> Result<Arc<dyn BackendQueue>>;

    fn create_buffer(
        self: Arc<Self>,
        create_info: &MagmaCreateBufferInfo,
    ) -> Result<Arc<dyn BackendBuffer>>;

    fn import(self: Arc<Self>, _info: MagmaImportHandleInfo) -> Result<Arc<dyn BackendBuffer>>;

    fn create_semaphore(
        self: Arc<Self>,
        _info: &MagmaCreateSemaphoreInfo,
    ) -> Result<Arc<dyn BackendSemaphore>> {
        Err(Error::Unimplemented)
    }

    fn import_semaphore(
        self: Arc<Self>,
        _info: MagmaImportHandleInfo,
    ) -> Result<Arc<dyn BackendSemaphore>> {
        Err(Error::Unimplemented)
    }
}

pub trait GenericBuffer {
    fn map(self: Arc<Self>) -> Result<Arc<dyn MappedRegion>>;

    fn export(&self) -> Result<MagmaGpuHandle>;

    fn invalidate(&self, sync_flags: u64, ranges: &[MagmaMappedMemoryRange]) -> Result<()>;

    fn flush(&self, sync_flags: u64, ranges: &[MagmaMappedMemoryRange]) -> Result<()>;

    fn get_map_info(&self) -> Option<u32>;

    fn as_gem_handle(&self) -> Option<u32> {
        None
    }

    fn id(&self) -> Option<u32> {
        self.as_gem_handle()
    }

    fn object_id(&self) -> Option<u64> {
        self.id().map(u64::from)
    }

    fn pat_index(&self) -> u16 {
        0
    }

    fn record_submission(&self, _ring_idx: u32, _seqno: u32) {}
}

pub trait GenericSemaphore {
    fn get_type(&self) -> MagmaSyncType {
        MagmaSyncType::Binary
    }

    fn is_timeline(&self) -> bool {
        self.get_type() == MagmaSyncType::Timeline
    }

    /// Wait until signaled or `abs_timeout_ns` (absolute CLOCK_MONOTONIC deadline in ns).
    fn wait(&self, _abs_timeout_ns: u64) -> Result<()> {
        Err(Error::Unimplemented)
    }

    fn signal(&self) -> Result<()> {
        Err(Error::Unimplemented)
    }

    fn reset(&self) -> Result<()> {
        Err(Error::Unimplemented)
    }

    /// Wait until `point` is reached or `abs_timeout_ns` (absolute CLOCK_MONOTONIC deadline in ns).
    fn timeline_wait(&self, _point: u64, _abs_timeout_ns: u64, _flags: u32) -> Result<()> {
        Err(Error::Unimplemented)
    }

    fn timeline_signal(&self, _point: u64) -> Result<()> {
        Err(Error::Unimplemented)
    }

    fn timeline_query(&self) -> Result<u64> {
        Err(Error::Unimplemented)
    }

    fn export_fence(&self) -> Result<MagmaGpuHandle> {
        Err(Error::Unimplemented)
    }

    fn import(&self, _handle: MagmaGpuHandle) -> Result<()> {
        Err(Error::Unimplemented)
    }

    fn as_raw_handle(&self) -> Option<u32> {
        None
    }

    fn object_id(&self) -> Option<u64> {
        None
    }

    fn use_flags(&self) -> MagmaSemaphoreUseFlags {
        MagmaSemaphoreUseFlags::empty()
    }
}

pub trait GenericAddressSpace {
    fn as_vm_id(&self) -> Option<u32> {
        None
    }

    fn map_buffer_gpu(
        &self,
        _buffer: &Arc<dyn BackendBuffer>,
        _buffer_offset: u64,
        _gpu_va: u64,
        _size: u64,
        _flags: MagmaGpuMapFlags,
    ) -> Result<()> {
        Err(Error::Unimplemented)
    }

    fn unmap_buffer_gpu(&self, _gpu_va: u64, _size: u64) -> Result<()> {
        Err(Error::Unimplemented)
    }
}

pub trait GenericQueue {
    fn submit_command(&self, _submit_info: &MagmaSubmitInfo) -> Result<()> {
        Err(Error::Unimplemented)
    }

    fn check_status(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtioSemaphoreOp {
    Wait {
        ring_idx: u32,
        seqno: u32,
    },
    Signal {
        ring_idx: u32,
        seqno: u32,
        point: u64,
    },
}

pub trait VirtioSemaphore {
    fn host_object_id(&self) -> u32;
    fn guest_syncobj_handle(&self) -> u32;
    fn has_imported_fence(&self) -> bool;
    fn last_ring_idx(&self) -> u32 {
        0
    }
    fn record_submission(&self, _op: VirtioSemaphoreOp) {}
}

pub trait AsVirtioSemaphore {
    fn as_virtio_semaphore(&self) -> Option<&dyn VirtioSemaphore> {
        None
    }
}

pub trait BackendPhysicalDevice:
    PlatformPhysicalDevice + AsVirtGpu + GenericPhysicalDevice + Send + Sync
{
}
pub trait BackendDevice: GenericDevice + PlatformDevice + Send + Sync {}
pub trait BackendAddressSpace: GenericAddressSpace + Send + Sync {}
pub trait BackendQueue: GenericQueue + Send + Sync {}
pub trait BackendBuffer: GenericBuffer + Send + Sync {}
pub trait BackendSemaphore: GenericSemaphore + AsVirtioSemaphore + Send + Sync {}
