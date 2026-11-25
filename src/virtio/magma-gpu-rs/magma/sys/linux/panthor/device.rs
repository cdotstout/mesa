// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::os::fd::AsFd;
use std::sync::Arc;

use magma_gpu::util::OwnedDescriptor;
use zerocopy::IntoBytes;

use crate::defines::MagmaAddressSpaceProperties;
use crate::defines::MagmaCreateBufferInfo;
use crate::defines::MagmaCreateQueueInfo;
use crate::defines::MagmaCreateSemaphoreInfo;
use crate::defines::MagmaHeap;
use crate::defines::MagmaHeapBudget;
use crate::defines::MagmaImportHandleInfo;
use crate::defines::MagmaMemoryType;
use crate::defines::MagmaPhysicalDeviceInfo;
use crate::defines::MagmaQueueFamilyProperties;
use crate::defines::MagmaQueueFlags;
use crate::defines::MagmaSyncType;
use crate::defines::MagmaTimestampProperties;
use crate::error::Error;
use crate::error::Result;
use crate::sys::linux::bindings::panthor_bindings::drm_panthor_csif_info;
use crate::sys::linux::bindings::panthor_bindings::drm_panthor_gpu_info;
use crate::sys::linux::drm::DrmSemaphore;
use crate::sys::linux::panthor::ioctl;
use crate::sys::linux::panthor::memory::PanthorAddressSpace;
use crate::sys::linux::panthor::memory::PanthorBuffer;
use crate::sys::linux::panthor::queue::PanthorQueue;
use crate::sys::linux::LinuxDescriptor;
use crate::sys::linux::PlatformDevice;
use crate::sys::linux::PlatformPhysicalDevice;
use crate::traits::AsVirtGpu;
use crate::traits::BackendAddressSpace;
use crate::traits::BackendBuffer;
use crate::traits::BackendDevice;
use crate::traits::BackendPhysicalDevice;
use crate::traits::BackendQueue;
use crate::traits::BackendSemaphore;
use crate::traits::GenericDevice;
use crate::traits::GenericPhysicalDevice;
use crate::traits::MagmaVendorQueryResult;
use crate::vendor::arm::MagmaArmDeviceInfo;
use crate::vendor::arm::MAGMA_ARM_QUERY_DEVICE_INFO;
use crate::traits::GenericSemaphore;

#[allow(dead_code)]
#[derive(Debug)]
pub struct PanthorPhysicalDevice {
    descriptor: LinuxDescriptor,
    gpu_info: drm_panthor_gpu_info,
    csif_info: drm_panthor_csif_info,
}

pub struct Panthor {
    physical_device: Arc<PanthorPhysicalDevice>,
    mem_types: Vec<MagmaMemoryType>,
    mem_heaps: Vec<MagmaHeap>,
}

#[allow(dead_code)]
impl PanthorPhysicalDevice {
    pub fn new(descriptor: OwnedDescriptor) -> PanthorPhysicalDevice {
        let gpu_info = ioctl::query_gpu_info(descriptor.as_fd()).unwrap_or_default();
        let csif_info = ioctl::query_csif_info(descriptor.as_fd()).unwrap_or_default();
        PanthorPhysicalDevice {
            descriptor: LinuxDescriptor::new(descriptor),
            gpu_info,
            csif_info,
        }
    }

    pub fn gpu_info(&self) -> &drm_panthor_gpu_info {
        &self.gpu_info
    }

    pub fn csif_info(&self) -> &drm_panthor_csif_info {
        &self.csif_info
    }

    pub fn gpu_id(&self) -> u32 {
        self.gpu_info.gpu_id
    }

    pub fn device_id(&self) -> u16 {
        (self.gpu_info.gpu_id >> 12) as u16
    }
}

impl PlatformPhysicalDevice for PanthorPhysicalDevice {
    fn as_linux_descriptor(&self) -> Option<&LinuxDescriptor> {
        Some(&self.descriptor)
    }
}

impl AsVirtGpu for PanthorPhysicalDevice {}
impl BackendPhysicalDevice for PanthorPhysicalDevice {}

impl GenericPhysicalDevice for PanthorPhysicalDevice {
    fn create_device(
        self: Arc<PanthorPhysicalDevice>,
        _device_info: &MagmaPhysicalDeviceInfo,
    ) -> Result<Arc<dyn BackendDevice>> {
        Ok(Arc::new(Panthor::new(self)?))
    }

    fn get_memory_types(&self) -> Result<Vec<MagmaMemoryType>> {
        Ok(ioctl::query_memory_types(&self.gpu_info))
    }

    fn get_memory_heaps(&self) -> Result<Vec<MagmaHeap>> {
        Ok(ioctl::query_memory_heaps())
    }

    fn get_queue_family_properties(&self) -> Result<Vec<MagmaQueueFamilyProperties>> {
        Ok(vec![
            MagmaQueueFamilyProperties::new(
                MagmaQueueFlags::Graphics
                    | MagmaQueueFlags::Compute
                    | MagmaQueueFlags::Transfer
                    | MagmaQueueFlags::SparseBinding,
                2,
            ),
            MagmaQueueFamilyProperties::new(MagmaQueueFlags::SparseBinding, 1),
        ])
    }

    fn get_address_space_properties(&self) -> Result<MagmaAddressSpaceProperties> {
        let va_bits = self.gpu_info.mmu_features & 0xff;
        let va_size = if va_bits > 0 && va_bits < 64 {
            1u64 << va_bits
        } else {
            1u64 << 48
        };
        Ok(MagmaAddressSpaceProperties::new(0, va_size, 4096))
    }

    fn get_timestamp_properties(&self) -> Result<MagmaTimestampProperties> {
        let ts_info = ioctl::query_timestamp_info(self.descriptor.as_fd()).unwrap_or_default();
        Ok(MagmaTimestampProperties::new(ts_info.timestamp_frequency))
    }

    fn query_vendor(
        &self,
        query_id: u64,
        buffer: Option<&mut [u8]>,
    ) -> Result<MagmaVendorQueryResult> {
        match query_id {
            MAGMA_ARM_QUERY_DEVICE_INFO => {
                let g = &self.gpu_info;
                let c = &self.csif_info;
                let info = MagmaArmDeviceInfo {
                    shader_present: g.shader_present,
                    tiler_present: g.tiler_present,
                    l2_present: g.l2_present,
                    gpu_id: g.gpu_id,
                    gpu_rev: g.gpu_rev,
                    csf_id: g.csf_id,
                    l2_features: g.l2_features,
                    tiler_features: g.tiler_features,
                    mem_features: g.mem_features,
                    mmu_features: g.mmu_features,
                    thread_features: g.thread_features,
                    max_threads: g.max_threads,
                    thread_max_workgroup_size: g.thread_max_workgroup_size,
                    thread_max_barrier_size: g.thread_max_barrier_size,
                    coherency_features: g.coherency_features,
                    selected_coherency: g.selected_coherency,
                    as_present: g.as_present,
                    csg_slot_count: c.csg_slot_count,
                    cs_slot_count: c.cs_slot_count,
                    cs_reg_count: c.cs_reg_count,
                    scoreboard_slot_count: c.scoreboard_slot_count,
                };
                let bytes = info.as_bytes();
                let Some(buf) = buffer else {
                    return Ok(MagmaVendorQueryResult::Buffer(bytes.len()));
                };
                if buf.len() < bytes.len() {
                    return Err(Error::InvalidArgs);
                }
                buf[..bytes.len()].copy_from_slice(bytes);
                Ok(MagmaVendorQueryResult::Buffer(bytes.len()))
            }
            _ => Err(Error::Unimplemented),
        }
    }
}

impl Panthor {
    pub fn new(physical_device: Arc<PanthorPhysicalDevice>) -> Result<Panthor> {
        let mem_types = physical_device.get_memory_types()?;
        let mem_heaps = physical_device.get_memory_heaps()?;
        Ok(Panthor {
            physical_device,
            mem_types,
            mem_heaps,
        })
    }
}

impl PlatformDevice for Panthor {}
impl BackendDevice for Panthor {}

impl GenericDevice for Panthor {
    fn get_memory_budget(&self, heap_idx: u32) -> Result<MagmaHeapBudget> {
        if let Some(heap) = self.mem_heaps.get(heap_idx as usize) {
            Ok(MagmaHeapBudget {
                budget: heap.heap_size,
                usage: 0,
            })
        } else {
            Err(Error::InvalidArgs)
        }
    }

    fn create_address_space(self: Arc<Panthor>) -> Result<Arc<dyn BackendAddressSpace>> {
        Ok(Arc::new(PanthorAddressSpace::new(
            self.physical_device.clone(),
        )?))
    }

    fn create_queue(
        self: Arc<Panthor>,
        address_space: &Arc<dyn BackendAddressSpace>,
        info: &MagmaCreateQueueInfo,
    ) -> Result<Arc<dyn BackendQueue>> {
        let vm_id = address_space.as_vm_id().ok_or(Error::Unimplemented)?;
        Ok(Arc::new(PanthorQueue::new(
            self.physical_device.clone(),
            vm_id,
            info,
        )?))
    }

    fn create_buffer(
        self: Arc<Panthor>,
        create_info: &MagmaCreateBufferInfo,
    ) -> Result<Arc<dyn BackendBuffer>> {
        let buf = PanthorBuffer::new(self.physical_device.clone(), create_info, &self.mem_types)?;
        let buf: Arc<dyn BackendBuffer> = Arc::new(buf);
        self.physical_device.register_buffer(&buf);
        Ok(buf)
    }

    fn import(self: Arc<Panthor>, info: MagmaImportHandleInfo) -> Result<Arc<dyn BackendBuffer>> {
        let gem_handle = self.physical_device.import(info.handle)?;
        if let Some(existing) = self.physical_device.find_buffer(gem_handle) {
            return Ok(existing);
        }
        let mem_type = self
            .mem_types
            .get(info.memory_type_idx as usize)
            .ok_or(Error::InvalidArgs)?;
        let buf = PanthorBuffer::from_existing(
            self.physical_device.clone(),
            gem_handle,
            info.size.try_into()?,
            mem_type.get_map_info(),
        )?;
        let buf: Arc<dyn BackendBuffer> = Arc::new(buf);
        self.physical_device.register_buffer(&buf);
        Ok(buf)
    }

    fn create_semaphore(
        self: Arc<Panthor>,
        info: &MagmaCreateSemaphoreInfo,
    ) -> Result<Arc<dyn BackendSemaphore>> {
        let semaphore = DrmSemaphore::new_from_info(self.physical_device.clone(), info)?;
        Ok(Arc::new(semaphore))
    }

    fn import_semaphore(
        self: Arc<Panthor>,
        info: MagmaImportHandleInfo,
    ) -> Result<Arc<dyn BackendSemaphore>> {
        let semaphore = DrmSemaphore::new(self.physical_device.clone(), MagmaSyncType::Binary)?;
        semaphore.import(info.handle)?;
        Ok(Arc::new(semaphore))
    }
}
