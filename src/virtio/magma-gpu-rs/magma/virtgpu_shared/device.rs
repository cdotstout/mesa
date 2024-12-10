// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;

use magma_gpu::util::MappedRegion;
use zerocopy::IntoBytes;

use crate::defines::MagmaCreateBufferInfo;
use crate::defines::MagmaCreateQueueInfo;
use crate::defines::MagmaImportHandleInfo;
use crate::encoder::Encoder;
use crate::error::Error;
use crate::error::Result;
use crate::protocol::DeviceCreateAddressSpace;
use crate::protocol::DeviceCreateBuffer;
use crate::protocol::DeviceCreateQueue;
use crate::protocol::DeviceCreateSemaphore;
use crate::protocol::MagmaCommandHeader;
use crate::protocol::MagmaCreateSemaphoreInfo;
use crate::protocol::MagmaHeapBudget;
use crate::protocol::MagmaSemaphoreType;
use crate::protocol::MagmaVirtCapabilities;
use crate::protocol::PhysicalDeviceCreateDevice;
use crate::protocol::VirtioCreateRing;
use crate::protocol::VirtioImportBuffer;
use crate::protocol::VirtioPing;
use crate::protocol::MAGMA_OPCODE_DEVICE_CREATE_ADDRESS_SPACE;
use crate::protocol::MAGMA_OPCODE_DEVICE_CREATE_BUFFER;
use crate::protocol::MAGMA_OPCODE_DEVICE_CREATE_QUEUE;
use crate::protocol::MAGMA_OPCODE_DEVICE_CREATE_SEMAPHORE;
use crate::protocol::MAGMA_OPCODE_PHYSICAL_DEVICE_CREATE_DEVICE;
use crate::protocol::MAGMA_OPCODE_VIRTIO_CREATE_RING;
use crate::protocol::MAGMA_OPCODE_VIRTIO_IMPORT_BUFFER;
use crate::protocol::MAGMA_OPCODE_VIRTIO_PING;
use crate::ring::MagmaRingBuffer;
use crate::sys::platform::PlatformDevice;
use crate::traits::BackendAddressSpace;
use crate::traits::BackendBuffer;
use crate::traits::BackendDevice;
use crate::traits::BackendQueue;
use crate::traits::BackendSemaphore;
use crate::traits::GenericDevice;
use crate::virtgpu_shared::memory::VirtioGpuAddressSpace;
use crate::virtgpu_shared::memory::VirtioGpuBuffer;
use crate::virtgpu_shared::queue::VirtioGpuQueue;
use crate::virtgpu_shared::semaphore::SemaphoreTracker;
use crate::virtgpu_shared::semaphore::VirtioGpuSemaphore;
use crate::virtgpu_shared::validate_ring_buffer_len;
use crate::virtgpu_shared::VirtioGpuTransport;

pub struct VirtioGpu<T: VirtioGpuTransport> {
    pub transport: Arc<T>,
    pub caps: MagmaVirtCapabilities,
    pub object_id: u32,
    pub ring_counter: AtomicU32,
    pub cpu_ring: MagmaRingBuffer,
    pub _cpu_bo_handle: u32,
    pub _cpu_mapping: Arc<dyn MappedRegion>,
}

impl<T: VirtioGpuTransport> VirtioGpu<T> {
    pub fn new(transport: Arc<T>) -> Result<VirtioGpu<T>> {
        transport.init_context()?;

        let ring_size = validate_ring_buffer_len(transport.caps().ring_buffer_len)?;
        let (cpu_bo_handle, cpu_res_handle) = transport.create_blob(0, ring_size)?;
        let cpu_mapping = transport.map_blob(cpu_bo_handle, ring_size)?;
        let cpu_ring = unsafe { MagmaRingBuffer::from_raw_parts(cpu_mapping.as_ptr(), ring_size)? };

        let object_id = transport.next_object_id();

        let device = VirtioGpu {
            caps: *transport.caps(),
            transport,
            object_id,
            ring_counter: AtomicU32::new(0),
            cpu_ring,
            _cpu_bo_handle: cpu_bo_handle,
            _cpu_mapping: cpu_mapping,
        };

        device.create_ring(0, cpu_res_handle)?;

        let create_dev = PhysicalDeviceCreateDevice {
            header: MagmaCommandHeader {
                opcode: MAGMA_OPCODE_PHYSICAL_DEVICE_CREATE_DEVICE,
                size: std::mem::size_of::<PhysicalDeviceCreateDevice>() as u32,
            },
            physical_device: device.transport.pdev_idx(),
            device: device.object_id,
        };
        device.submit_encoded_cpu_cmd(create_dev.as_bytes())?;

        Ok(device)
    }

    pub fn create_ring(&self, ring_idx: u32, ring_blob_id: u32) -> Result<()> {
        let req = VirtioCreateRing {
            header: MagmaCommandHeader {
                opcode: MAGMA_OPCODE_VIRTIO_CREATE_RING,
                size: std::mem::size_of::<VirtioCreateRing>() as u32,
            },
            ring_idx,
            ring_blob_id,
        };
        self.submit_encoded_cmd(req.as_bytes(), ring_idx, &[], &[])
    }

    pub fn ping_ring(&self, ring_idx: u32) -> Result<()> {
        let req = VirtioPing {
            header: MagmaCommandHeader {
                opcode: MAGMA_OPCODE_VIRTIO_PING,
                size: std::mem::size_of::<VirtioPing>() as u32,
            },
            ring_idx,
            _padding: 0,
        };
        self.submit_encoded_cmd(req.as_bytes(), ring_idx, &[], &[])
    }

    pub fn submit_encoded_cmd(
        &self,
        cmd: &[u8],
        ring_idx: u32,
        bo_handles: &[u32],
        syncobjs: &[u32],
    ) -> Result<()> {
        self.transport.submit_cmd(cmd, ring_idx, bo_handles, syncobjs)
    }

    pub fn submit_encoded_cpu_cmd(&self, cmd: &[u8]) -> Result<()> {
        self.cpu_ring
            .write_and_ping_locked(cmd, || self.ping_ring(0))
    }
}

impl<T: VirtioGpuTransport> PlatformDevice for VirtioGpu<T> {}
impl<T: VirtioGpuTransport> BackendDevice for VirtioGpu<T> {}

impl<T: VirtioGpuTransport> GenericDevice for VirtioGpu<T> {
    fn get_memory_budget(&self, heap_idx: u32) -> Result<MagmaHeapBudget> {
        if let Some(heap) = self.transport.device().memory_heaps.get(heap_idx as usize) {
            Ok(MagmaHeapBudget {
                budget: heap.heap_size,
                usage: 0,
            })
        } else {
            Err(Error::InvalidArgs)
        }
    }

    fn create_address_space(self: Arc<VirtioGpu<T>>) -> Result<Arc<dyn BackendAddressSpace>> {
        let object_id = self.transport.next_object_id();
        let req = DeviceCreateAddressSpace {
            header: MagmaCommandHeader {
                opcode: MAGMA_OPCODE_DEVICE_CREATE_ADDRESS_SPACE,
                size: std::mem::size_of::<DeviceCreateAddressSpace>() as u32,
            },
            device: self.object_id,
            address_space: object_id,
        };
        self.submit_encoded_cpu_cmd(req.as_bytes())?;
        Ok(Arc::new(VirtioGpuAddressSpace {
            device: self,
            object_id,
        }))
    }

    fn create_queue(
        self: Arc<VirtioGpu<T>>,
        address_space: &Arc<dyn BackendAddressSpace>,
        info: &MagmaCreateQueueInfo,
    ) -> Result<Arc<dyn BackendQueue>> {
        let object_id = self.transport.next_object_id();
        let as_id = address_space.as_vm_id().ok_or(Error::InvalidArgs)?;
        let req = DeviceCreateQueue {
            header: MagmaCommandHeader {
                opcode: MAGMA_OPCODE_DEVICE_CREATE_QUEUE,
                size: std::mem::size_of::<DeviceCreateQueue>() as u32,
            },
            device: self.object_id,
            address_space: as_id,
            info: *info,
            queue: object_id,
            _padding: 0,
        };
        self.submit_encoded_cpu_cmd(req.as_bytes())?;

        let ring_size = validate_ring_buffer_len(self.caps.ring_buffer_len)?;
        let (bo_handle, res_handle) = self.transport.create_blob(0, ring_size)?;
        let mapping = self.transport.map_blob(bo_handle, ring_size)?;
        let ring = unsafe { MagmaRingBuffer::from_raw_parts(mapping.as_ptr(), ring_size)? };

        let ring_idx = (self.ring_counter.fetch_add(1, Ordering::Relaxed) % 63) + 1;

        self.create_ring(ring_idx, res_handle)?;

        Ok(Arc::new(VirtioGpuQueue {
            device: self,
            object_id,
            ring_idx,
            ring,
            _bo_handle: bo_handle,
            _mapping: mapping,
        }))
    }

    fn create_buffer(
        self: Arc<VirtioGpu<T>>,
        create_info: &MagmaCreateBufferInfo,
    ) -> Result<Arc<dyn BackendBuffer>> {
        let object_id = self.transport.next_object_id();
        let map_info = self
            .transport
            .device()
            .memory_types
            .get(create_info.memory_type_idx as usize)
            .and_then(|mt| mt.get_map_info());

        let cmd = DeviceCreateBuffer {
            header: MagmaCommandHeader {
                opcode: MAGMA_OPCODE_DEVICE_CREATE_BUFFER,
                size: std::mem::size_of::<DeviceCreateBuffer>() as u32,
            },
            device: self.object_id,
            _pad0: 0,
            info: *create_info,
            buffer_out: object_id,
            _padding: 0,
        };
        let mut buf = [0u8; std::mem::size_of::<DeviceCreateBuffer>()];
        let mut encoder = Encoder::from_slice(&mut buf);
        encoder.encode_device_create_buffer(&cmd)?;
        let written = encoder.bytes_written();
        self.submit_encoded_cpu_cmd(&buf[..written])?;

        Ok(Arc::new_cyclic(|weak| VirtioGpuBuffer {
            device: self,
            object_id,
            bo_handle: AtomicU32::new(0),
            _res_handle: AtomicU32::new(0),
            blob_lock: Mutex::new(()),
            size: create_info.size as usize,
            map_info,
            ring_submission: AtomicU64::new(0),
            weak_self: weak.clone(),
        }))
    }

    fn import(
        self: Arc<VirtioGpu<T>>,
        info: MagmaImportHandleInfo,
    ) -> Result<Arc<dyn BackendBuffer>> {
        let (bo_handle, res_handle, imported_size) = self.transport.import_blob(info.handle)?;
        if let Some(existing) = self.transport.find_buffer(bo_handle) {
            return Ok(existing);
        }
        let size = if info.size > 0 {
            info.size as usize
        } else {
            imported_size
        };
        let object_id = self.transport.next_object_id();
        let map_info = self
            .transport
            .device()
            .memory_types
            .get(info.memory_type_idx as usize)
            .and_then(|mt| mt.get_map_info());

        let req = VirtioImportBuffer {
            header: MagmaCommandHeader {
                opcode: MAGMA_OPCODE_VIRTIO_IMPORT_BUFFER,
                size: std::mem::size_of::<VirtioImportBuffer>() as u32,
            },
            res_handle,
            buffer_out: object_id,
            size: size as u64,
            memory_type_idx: info.memory_type_idx,
            _padding: 0,
        };

        // The host registers `object_id` while processing this execbuffer on the virtio-gpu
        // control queue, but later commands on this buffer (MapBuffer, QueueSubmit, ...)
        // travel on the shared-memory rings. Wait on the blob's fence so that the ring
        // workers never race ahead of control-queue import completion.
        let res = self
            .submit_encoded_cmd(req.as_bytes(), 0, &[bo_handle], &[])
            .and_then(|_| self.transport.wait_blob(bo_handle));
        if let Err(e) = res {
            self.transport.close_blob(bo_handle);
            return Err(e);
        }

        let buf: Arc<dyn BackendBuffer> = Arc::new_cyclic(|weak| VirtioGpuBuffer {
            device: self.clone(),
            object_id,
            bo_handle: AtomicU32::new(bo_handle),
            _res_handle: AtomicU32::new(res_handle),
            blob_lock: Mutex::new(()),
            size,
            map_info,
            ring_submission: AtomicU64::new(0),
            weak_self: weak.clone(),
        });
        self.transport.register_buffer(&buf);
        Ok(buf)
    }

    fn create_semaphore(
        self: Arc<VirtioGpu<T>>,
        info: &MagmaCreateSemaphoreInfo,
    ) -> Result<Arc<dyn BackendSemaphore>> {
        let object_id = self.transport.next_object_id();
        let cmd = DeviceCreateSemaphore {
            header: MagmaCommandHeader {
                opcode: MAGMA_OPCODE_DEVICE_CREATE_SEMAPHORE,
                size: std::mem::size_of::<DeviceCreateSemaphore>() as u32,
            },
            device: self.object_id,
            _pad0: 0,
            info: *info,
            semaphore: object_id,
            _padding: 0,
        };
        let mut buf = [0u8; std::mem::size_of::<DeviceCreateSemaphore>()];
        let mut encoder = Encoder::from_slice(&mut buf);
        encoder.encode_device_create_semaphore(&cmd)?;
        let written = encoder.bytes_written();
        self.submit_encoded_cpu_cmd(&buf[..written])?;

        let sync_type = MagmaSemaphoreType::from(info.semaphore_type);
        let guest_syncobj_handle = self.transport.syncobj_create(info.initial_point > 0)?;

        Ok(Arc::new(VirtioGpuSemaphore {
            device: self,
            object_id,
            guest_syncobj_handle,
            use_flags: info.use_flags,
            sync_type,
            tracker: SemaphoreTracker::new(info.initial_point),
        }))
    }
}
