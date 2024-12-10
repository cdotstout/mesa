// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use magma_gpu::util::MappedRegion;

use crate::defines::MagmaStructureType;
use crate::defines::MagmaStructureTypeHeader;
use crate::defines::MagmaSubmitInfo;
use crate::defines::MagmaSubmitPayload;
use crate::encoder::Encoder;
use crate::error::Error;
use crate::error::Result;
use crate::protocol::MagmaCommandHeader;
use crate::protocol::MagmaSubmitAddressSpaceInfo;
use crate::protocol::MagmaSubmitUnmapBufferInfo;
use crate::protocol::MagmaVirtioSeqnoInfo;
use crate::protocol::MagmaWireSubmitBufferInfo;
use crate::protocol::MagmaWireSubmitInfo;
use crate::protocol::MagmaWireSubmitInfoHeader;
use crate::protocol::MagmaWireSubmitMapBufferInfo;
use crate::protocol::MagmaWireSubmitSyncInfo;
use crate::protocol::MagmaWireSubmitSyncInfoHeader;
use crate::protocol::QueueSubmit;
use crate::protocol::QueueSubmitHeader;
use crate::protocol::MAGMA_OPCODE_QUEUE_SUBMIT;
use crate::ring::MagmaRingBuffer;
use crate::traits::BackendQueue;
use crate::traits::GenericQueue;
use crate::traits::VirtioSemaphoreOp;
use crate::virtgpu_shared::device::VirtioGpu;
use crate::virtgpu_shared::VirtioGpuTransport;

enum WireSubmitExt {
    VirtioSeqno(MagmaVirtioSeqnoInfo),
    AddressSpace(MagmaSubmitAddressSpaceInfo),
    Buffer(MagmaWireSubmitBufferInfo),
    MapBuffer(MagmaWireSubmitMapBufferInfo),
    UnmapBuffer(MagmaSubmitUnmapBufferInfo),
}

impl WireSubmitExt {
    fn header_ptr(&self) -> u64 {
        match self {
            WireSubmitExt::VirtioSeqno(info) => &info.header as *const _ as u64,
            WireSubmitExt::AddressSpace(info) => &info.header as *const _ as u64,
            WireSubmitExt::Buffer(info) => &info.header as *const _ as u64,
            WireSubmitExt::MapBuffer(info) => &info.header as *const _ as u64,
            WireSubmitExt::UnmapBuffer(info) => &info.header as *const _ as u64,
        }
    }

    fn set_p_next(&mut self, p_next: u64) {
        match self {
            WireSubmitExt::VirtioSeqno(info) => info.header.p_next = p_next,
            WireSubmitExt::AddressSpace(info) => info.header.p_next = p_next,
            WireSubmitExt::Buffer(info) => info.header.p_next = p_next,
            WireSubmitExt::MapBuffer(info) => info.header.p_next = p_next,
            WireSubmitExt::UnmapBuffer(info) => info.header.p_next = p_next,
        }
    }
}

pub struct VirtioGpuQueue<T: VirtioGpuTransport> {
    pub device: Arc<VirtioGpu<T>>,
    pub object_id: u32,
    pub ring_idx: u32,
    pub ring: MagmaRingBuffer,
    pub _bo_handle: u32,
    pub _mapping: Arc<dyn MappedRegion>,
}

impl<T: VirtioGpuTransport> GenericQueue for VirtioGpuQueue<T> {
    fn submit_command(&self, submit_info: &MagmaSubmitInfo) -> Result<()> {
        let mut wire_sync_info = MagmaWireSubmitSyncInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitSyncInfo as u32,
                size: std::mem::size_of::<MagmaWireSubmitSyncInfoHeader>() as u32,
                p_next: 0,
            },
            num_wait_semaphores: 0,
            num_signal_semaphores: 0,
            wait_semaphores: Vec::new(),
            signal_semaphores: Vec::new(),
            wait_points: Vec::new(),
            signal_points: Vec::new(),
        };

        for (semaphore, wait_point) in submit_info.sync_info.wait_semaphores_with_points() {
            if let Some(virtio_sem) = semaphore.as_virtio_semaphore() {
                if virtio_sem.has_imported_fence() {
                    semaphore.wait(u64::MAX)?;
                } else {
                    wire_sync_info
                        .wait_semaphores
                        .push(virtio_sem.host_object_id());
                    wire_sync_info.wait_points.push(wait_point);
                    wire_sync_info.num_wait_semaphores += 1;
                }
            }
        }

        for (semaphore, signal_point) in submit_info.sync_info.signal_semaphores_with_points() {
            if let Some(virtio_sem) = semaphore.as_virtio_semaphore() {
                wire_sync_info
                    .signal_semaphores
                    .push(virtio_sem.host_object_id());
                wire_sync_info.signal_points.push(signal_point);
                wire_sync_info.num_signal_semaphores += 1;
            }
        }

        let mut wire_exts = Vec::new();
        let mut has_exec_payload = false;
        for payload in submit_info.payloads() {
            match payload {
                MagmaSubmitPayload::AddressSpace(as_info) => {
                    has_exec_payload = true;
                    let mut wire_as = *as_info;
                    wire_as.header.p_next = 0;
                    wire_exts.push(WireSubmitExt::AddressSpace(wire_as));
                }
                MagmaSubmitPayload::Buffer(buf_info) => {
                    has_exec_payload = true;
                    let command_buffer = buf_info
                        .command_buffer_handle()
                        .ok_or(Error::InvalidArgs)?;
                    wire_exts.push(WireSubmitExt::Buffer(MagmaWireSubmitBufferInfo {
                        header: MagmaStructureTypeHeader {
                            stype: MagmaStructureType::SubmitBufferInfo as u32,
                            size: std::mem::size_of::<MagmaWireSubmitBufferInfo>() as u32,
                            p_next: 0,
                        },
                        command_buffer,
                        _pad0: 0,
                        start_offset: buf_info.start_offset,
                        length: buf_info.length,
                    }));
                }
                MagmaSubmitPayload::MapBuffer(map_info) => {
                    let buffer = map_info.buffer_handle().ok_or(Error::InvalidArgs)?;
                    wire_exts.push(WireSubmitExt::MapBuffer(MagmaWireSubmitMapBufferInfo {
                        header: MagmaStructureTypeHeader {
                            stype: MagmaStructureType::SubmitMapBufferInfo as u32,
                            size: std::mem::size_of::<MagmaWireSubmitMapBufferInfo>() as u32,
                            p_next: 0,
                        },
                        buffer,
                        _pad0: 0,
                        buffer_offset: map_info.buffer_offset,
                        gpu_va: map_info.gpu_va,
                        size: map_info.size,
                        flags: map_info.flags,
                    }));
                }
                MagmaSubmitPayload::UnmapBuffer(unmap_info) => {
                    let mut wire_unmap = *unmap_info;
                    wire_unmap.header.p_next = 0;
                    wire_exts.push(WireSubmitExt::UnmapBuffer(wire_unmap));
                }
                _ => return Err(Error::Unimplemented),
            }
        }

        let use_cpu_ring = !has_exec_payload
            && !wire_exts.is_empty()
            && wire_sync_info.num_wait_semaphores == 0
            && wire_sync_info.num_signal_semaphores == 0;

        if !use_cpu_ring {
            let wait_seqno = self.device.cpu_ring.submitted_seqno();
            if wait_seqno > 0 {
                wire_exts.insert(
                    0,
                    WireSubmitExt::VirtioSeqno(MagmaVirtioSeqnoInfo::new(0, wait_seqno)),
                );
            }
        }

        for i in (0..wire_exts.len()).rev() {
            let next_ptr = if i + 1 < wire_exts.len() {
                wire_exts[i + 1].header_ptr()
            } else {
                0
            };
            wire_exts[i].set_p_next(next_ptr);
        }

        let wire_submit_info = MagmaWireSubmitInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitInfo as u32,
                size: std::mem::size_of::<MagmaWireSubmitInfoHeader>() as u32,
                p_next: wire_exts.first().map_or(0, |ext| ext.header_ptr()),
            },
            flags: submit_info.flags,
            _pad0: 0,
            sync_info: wire_sync_info,
        };

        let dynamic_bytes = (wire_submit_info.sync_info.wait_semaphores.len()
            + wire_submit_info.sync_info.signal_semaphores.len())
            * (std::mem::size_of::<u32>() + std::mem::size_of::<u64>());

        let cmd = QueueSubmit {
            header: MagmaCommandHeader {
                opcode: MAGMA_OPCODE_QUEUE_SUBMIT,
                size: (std::mem::size_of::<QueueSubmitHeader>() + dynamic_bytes) as u32,
            },
            queue: self.object_id,
            _pad0: 0,
            submit_info: wire_submit_info,
        };

        let mut buf = Vec::with_capacity(
            std::mem::size_of::<QueueSubmitHeader>()
                + dynamic_bytes
                + wire_exts.len() * std::mem::size_of::<MagmaWireSubmitMapBufferInfo>(),
        );
        let mut encoder = Encoder::new(&mut buf);
        encoder.encode_queue_submit(&cmd)?;
        let _ = &wire_exts;

        if use_cpu_ring {
            return self.device.submit_encoded_cpu_cmd(&buf);
        }

        self.ring.write_and_ping_locked(&buf, || {
            self.device.ping_ring(self.ring_idx)
        })?;

        let submitted_seqno = self.ring.submitted_seqno();
        for payload in submit_info.payloads() {
            match payload {
                MagmaSubmitPayload::Buffer(buf_info) => {
                    if let Some(buf) = buf_info.buffer() {
                        buf.inner().record_submission(self.ring_idx, submitted_seqno);
                    }
                }
                MagmaSubmitPayload::MapBuffer(map_info) => {
                    if let Some(buf) = map_info.buffer() {
                        buf.inner().record_submission(self.ring_idx, submitted_seqno);
                    }
                }
                _ => {}
            }
        }
        for (semaphore, _) in submit_info.sync_info.wait_semaphores_with_points() {
            if let Some(virtio_sem) = semaphore.as_virtio_semaphore() {
                if !virtio_sem.has_imported_fence() {
                    virtio_sem.record_submission(VirtioSemaphoreOp::Wait {
                        ring_idx: self.ring_idx,
                        seqno: submitted_seqno,
                    });
                }
            }
        }
        for (semaphore, signal_point) in submit_info.sync_info.signal_semaphores_with_points() {
            if let Some(virtio_sem) = semaphore.as_virtio_semaphore() {
                virtio_sem.record_submission(VirtioSemaphoreOp::Signal {
                    ring_idx: self.ring_idx,
                    seqno: submitted_seqno,
                    point: signal_point,
                });
            }
        }

        Ok(())
    }
}

impl<T: VirtioGpuTransport> BackendQueue for VirtioGpuQueue<T> {}
