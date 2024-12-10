// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;

use magma_gpu::util::Handle as MagmaGpuHandle;
use magma_gpu::util::MappedRegion;
use zerocopy::IntoBytes;

use crate::defines::MagmaMappedMemoryRange;
use crate::error::Error;
use crate::error::Result;
use crate::protocol::MagmaCommandHeader;
use crate::protocol::VirtioBufferRelease;
use crate::protocol::MAGMA_OPCODE_VIRTIO_BUFFER_RELEASE;
use crate::traits::BackendAddressSpace;
use crate::traits::BackendBuffer;
use crate::traits::GenericAddressSpace;
use crate::traits::GenericBuffer;
use crate::virtgpu_shared::device::VirtioGpu;
use crate::virtgpu_shared::VirtioGpuTransport;

pub struct VirtioGpuAddressSpace<T: VirtioGpuTransport> {
    pub device: Arc<VirtioGpu<T>>,
    pub object_id: u32,
}

pub struct VirtioGpuBuffer<T: VirtioGpuTransport> {
    pub device: Arc<VirtioGpu<T>>,
    pub object_id: u32,
    pub bo_handle: AtomicU32,
    pub _res_handle: AtomicU32,
    pub blob_lock: Mutex<()>,
    pub size: usize,
    pub map_info: Option<u32>,
    pub ring_submission: AtomicU64,
    pub weak_self: Weak<VirtioGpuBuffer<T>>,
}

impl<T: VirtioGpuTransport> GenericAddressSpace for VirtioGpuAddressSpace<T> {
    fn as_vm_id(&self) -> Option<u32> {
        Some(self.object_id)
    }
}

impl<T: VirtioGpuTransport> BackendAddressSpace for VirtioGpuAddressSpace<T> {}

impl<T: VirtioGpuTransport> VirtioGpuBuffer<T> {
    fn ensure_blob(&self) -> Result<u32> {
        let handle = self.bo_handle.load(Ordering::Acquire);
        if handle != 0 {
            return Ok(handle);
        }
        let _guard = self.blob_lock.lock().map_err(|_| Error::InternalError)?;
        let handle = self.bo_handle.load(Ordering::Acquire);
        if handle != 0 {
            return Ok(handle);
        }
        let (bo_handle, res_handle) = self
            .device
            .transport
            .create_blob(self.object_id as u64, self.size)?;
        self.bo_handle.store(bo_handle, Ordering::Release);
        self._res_handle.store(res_handle, Ordering::Release);
        if let Some(arc) = self.weak_self.upgrade() {
            let buf: Arc<dyn BackendBuffer> = arc;
            self.device.transport.register_buffer(&buf);
        }
        Ok(bo_handle)
    }
}

impl<T: VirtioGpuTransport> GenericBuffer for VirtioGpuBuffer<T> {
    fn map(self: Arc<VirtioGpuBuffer<T>>) -> Result<Arc<dyn MappedRegion>> {
        let bo_handle = self.ensure_blob()?;
        self.device.transport.map_blob(bo_handle, self.size)
    }

    fn export(&self) -> Result<MagmaGpuHandle> {
        let bo_handle = self.ensure_blob()?;
        self.device.transport.export_blob(bo_handle)
    }

    fn invalidate(&self, _sync_flags: u64, _ranges: &[MagmaMappedMemoryRange]) -> Result<()> {
        Ok(())
    }

    fn flush(&self, _sync_flags: u64, _ranges: &[MagmaMappedMemoryRange]) -> Result<()> {
        Ok(())
    }

    fn as_gem_handle(&self) -> Option<u32> {
        let bo_handle = self.bo_handle.load(Ordering::Acquire);
        if bo_handle != 0 {
            Some(bo_handle)
        } else {
            None
        }
    }

    fn id(&self) -> Option<u32> {
        Some(self.object_id)
    }

    fn get_map_info(&self) -> Option<u32> {
        self.map_info
    }

    fn record_submission(&self, ring_idx: u32, seqno: u32) {
        if ring_idx == 0 || seqno == 0 {
            return;
        }
        let packed = ((ring_idx as u64) << 32) | (seqno as u64);
        self.ring_submission.store(packed, Ordering::Release);
    }
}

impl<T: VirtioGpuTransport> Drop for VirtioGpuBuffer<T> {
    fn drop(&mut self) {
        let bo_handle = self.bo_handle.load(Ordering::Acquire);
        if bo_handle != 0 {
            self.device.transport.close_blob(bo_handle);
        }
        let packed = self.ring_submission.load(Ordering::Acquire);
        let ring_idx = (packed >> 32) as u32;
        let wait_seqno = packed as u32;
        let req = VirtioBufferRelease {
            header: MagmaCommandHeader {
                opcode: MAGMA_OPCODE_VIRTIO_BUFFER_RELEASE,
                size: std::mem::size_of::<VirtioBufferRelease>() as u32,
            },
            buffer: self.object_id,
            ring_idx,
            wait_seqno,
            _padding: 0,
        };
        let _ = self.device.submit_encoded_cpu_cmd(req.as_bytes());
    }
}

impl<T: VirtioGpuTransport> BackendBuffer for VirtioGpuBuffer<T> {}
