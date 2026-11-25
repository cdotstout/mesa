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
use crate::sys::linux::drm::DrmSemaphore;
use crate::sys::linux::xe::ioctl::xe_query_config;
use crate::sys::linux::xe::ioctl::xe_query_gt_list;
use crate::sys::linux::xe::ioctl::xe_query_gt_topology;
use crate::sys::linux::xe::ioctl::xe_query_hwconfig;
use crate::sys::linux::xe::ioctl::xe_query_memory;
use crate::sys::linux::xe::ioctl::xe_query_memory_regions;
use crate::sys::linux::xe::memory::XeAddressSpace;
use crate::sys::linux::xe::memory::XeBuffer;
use crate::sys::linux::xe::queue::XeQueue;
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
use crate::traits::GenericSemaphore;
use crate::traits::MagmaVendorQueryResult;
use crate::vendor::intel::MagmaIntelDeviceInfo;
use crate::vendor::intel::MAGMA_INTEL_QUERY_DEVICE_INFO;
use crate::vendor::intel::MAGMA_INTEL_QUERY_HWCONFIG;

#[derive(Debug)]
pub struct XePhysicalDevice {
    descriptor: LinuxDescriptor,
}

pub struct Xe {
    physical_device: Arc<dyn BackendPhysicalDevice>,
    ip_ver_major: u16,
    ip_ver_minor: u16,
    _gtt_size: u64,
    _mem_alignment: u64,
    mem_types: Vec<MagmaMemoryType>,
    mem_heaps: Vec<MagmaHeap>,
    sysmem_instance: u16,
    vram_instance: u16,
}

impl XePhysicalDevice {
    pub fn new(descriptor: OwnedDescriptor) -> XePhysicalDevice {
        XePhysicalDevice {
            descriptor: LinuxDescriptor::new(descriptor),
        }
    }
}

impl PlatformPhysicalDevice for XePhysicalDevice {
    fn as_linux_descriptor(&self) -> Option<&LinuxDescriptor> {
        Some(&self.descriptor)
    }
}

impl AsVirtGpu for XePhysicalDevice {}
impl BackendPhysicalDevice for XePhysicalDevice {}

impl GenericPhysicalDevice for XePhysicalDevice {
    fn create_device(
        self: Arc<XePhysicalDevice>,
        info: &MagmaPhysicalDeviceInfo,
    ) -> Result<Arc<dyn BackendDevice>> {
        Ok(Arc::new(Xe::new(self, info)?))
    }

    fn get_memory_types(&self) -> Result<Vec<MagmaMemoryType>> {
        xe_query_memory(self.descriptor.as_fd()).map(|(types, _)| types)
    }

    fn get_memory_heaps(&self) -> Result<Vec<MagmaHeap>> {
        xe_query_memory(self.descriptor.as_fd()).map(|(_, heaps)| heaps)
    }

    fn get_queue_family_properties(&self) -> Result<Vec<MagmaQueueFamilyProperties>> {
        Ok(vec![
            MagmaQueueFamilyProperties::new(
                MagmaQueueFlags::Graphics
                    | MagmaQueueFlags::Compute
                    | MagmaQueueFlags::Transfer
                    | MagmaQueueFlags::SparseBinding
                    | MagmaQueueFlags::Protected,
                1,
            ),
            MagmaQueueFamilyProperties::new(
                MagmaQueueFlags::Compute
                    | MagmaQueueFlags::Transfer
                    | MagmaQueueFlags::SparseBinding,
                1,
            ),
            MagmaQueueFamilyProperties::new(
                MagmaQueueFlags::Transfer | MagmaQueueFlags::Protected,
                1,
            ),
        ])
    }

    fn get_address_space_properties(&self) -> Result<MagmaAddressSpaceProperties> {
        let (gtt_size, mem_alignment, _) = xe_query_config(self.descriptor.as_fd())?;
        Ok(MagmaAddressSpaceProperties::new(0, gtt_size, mem_alignment))
    }

    fn get_timestamp_properties(&self) -> Result<MagmaTimestampProperties> {
        let (reference_clock, _, _, _) = xe_query_gt_list(self.descriptor.as_fd())?;
        Ok(MagmaTimestampProperties::new(reference_clock as u64))
    }

    fn query_vendor(
        &self,
        query_id: u64,
        buffer: Option<&mut [u8]>,
    ) -> Result<MagmaVendorQueryResult> {
        let fd = self.descriptor.as_fd();
        match query_id {
            MAGMA_INTEL_QUERY_DEVICE_INFO => {
                let (_, _, raw_flags) = xe_query_config(fd)?;
                let (_, ip_ver_major, ip_ver_minor, ip_ver_rev) = xe_query_gt_list(fd)?;
                let (geo_dss_mask, geo_dss_num_bytes, eu_per_dss_mask, l3_banks) =
                    xe_query_gt_topology(fd)?;
                let info = MagmaIntelDeviceInfo {
                    l3_banks,
                    geo_dss_num_bytes,
                    geo_dss_mask,
                    eu_per_dss_mask,
                    ip_ver_major: ip_ver_major as u32,
                    ip_ver_minor: ip_ver_minor as u32,
                    ip_ver_rev: ip_ver_rev as u32,
                    config_flags: raw_flags,
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
            MAGMA_INTEL_QUERY_HWCONFIG => {
                let hwconfig_vec = xe_query_hwconfig(fd)?;
                let bytes = hwconfig_vec.as_bytes();
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

impl Xe {
    pub fn new(
        physical_device: Arc<dyn BackendPhysicalDevice>,
        _info: &MagmaPhysicalDeviceInfo,
    ) -> Result<Xe> {
        let fd = physical_device.as_fd().unwrap();
        let (_, ip_ver_major, ip_ver_minor, _) = xe_query_gt_list(fd)?;
        let (gtt_size, mem_alignment, _) = xe_query_config(fd)?;
        let memory_info = xe_query_memory_regions(fd)?;
        let (mem_types, mem_heaps) = xe_query_memory(fd)?;

        Ok(Xe {
            physical_device,
            ip_ver_major,
            ip_ver_minor,
            _gtt_size: gtt_size,
            _mem_alignment: mem_alignment,
            mem_types,
            mem_heaps,
            sysmem_instance: memory_info.sysmem_instance,
            vram_instance: memory_info.vram_instance,
        })
    }
}

impl GenericDevice for Xe {
    fn get_memory_budget(&self, heap_idx: u32) -> Result<MagmaHeapBudget> {
        if heap_idx >= self.mem_heaps.len() as u32 {
            return Err(Error::InvalidArgs);
        }

        let fd = self.physical_device.as_fd().unwrap();
        let memory_info = xe_query_memory_regions(fd)?;
        let heap = &self.mem_heaps[heap_idx as usize];

        let (budget, usage) = if heap.is_device_local() && heap.is_cpu_visible() {
            (
                memory_info.vram_cpu_visible_size,
                memory_info.vram_cpu_visible_used,
            )
        } else if heap.is_device_local() {
            (memory_info.vram_size, memory_info.vram_used)
        } else if heap.is_cpu_visible() {
            (memory_info.sysmem_size, memory_info.sysmem_used)
        } else {
            return Err(Error::Unimplemented);
        };

        Ok(MagmaHeapBudget { budget, usage })
    }

    fn create_address_space(self: Arc<Xe>) -> Result<Arc<dyn BackendAddressSpace>> {
        let addr_space = XeAddressSpace::new(self.physical_device.clone(), 0)?;
        Ok(Arc::new(addr_space))
    }

    fn create_queue(
        self: Arc<Xe>,
        address_space: &Arc<dyn BackendAddressSpace>,
        info: &MagmaCreateQueueInfo,
    ) -> Result<Arc<dyn BackendQueue>> {
        let queue = XeQueue::new(self.physical_device.clone(), address_space, info)?;
        Ok(Arc::new(queue))
    }

    fn create_buffer(
        self: Arc<Xe>,
        create_info: &MagmaCreateBufferInfo,
    ) -> Result<Arc<dyn BackendBuffer>> {
        let buf = XeBuffer::new(
            self.physical_device.clone(),
            self.ip_ver_major,
            self.ip_ver_minor,
            create_info,
            &self.mem_types,
            &self.mem_heaps,
            self.sysmem_instance,
            self.vram_instance,
        )?;
        let buf: Arc<dyn BackendBuffer> = Arc::new(buf);
        self.physical_device.register_buffer(&buf);
        Ok(buf)
    }

    fn import(self: Arc<Xe>, info: MagmaImportHandleInfo) -> Result<Arc<dyn BackendBuffer>> {
        let gem_handle = self.physical_device.import(info.handle)?;
        if let Some(existing) = self.physical_device.find_buffer(gem_handle) {
            return Ok(existing);
        }
        let memory_type = self
            .mem_types
            .get(info.memory_type_idx as usize)
            .ok_or(Error::InvalidArgs)?;
        let buf = XeBuffer::from_existing(
            self.physical_device.clone(),
            self.ip_ver_major,
            self.ip_ver_minor,
            gem_handle,
            info.size.try_into()?,
            memory_type.get_map_info(),
            memory_type.is_cached(),
        )?;
        let buf: Arc<dyn BackendBuffer> = Arc::new(buf);
        self.physical_device.register_buffer(&buf);
        Ok(buf)
    }

    fn create_semaphore(
        self: Arc<Xe>,
        info: &MagmaCreateSemaphoreInfo,
    ) -> Result<Arc<dyn BackendSemaphore>> {
        let semaphore = DrmSemaphore::new_from_info(self.physical_device.clone(), info)?;
        Ok(Arc::new(semaphore))
    }

    fn import_semaphore(
        self: Arc<Xe>,
        info: MagmaImportHandleInfo,
    ) -> Result<Arc<dyn BackendSemaphore>> {
        let semaphore = DrmSemaphore::new(self.physical_device.clone(), MagmaSyncType::Binary)?;
        semaphore.import(info.handle)?;
        Ok(Arc::new(semaphore))
    }
}

impl PlatformDevice for Xe {}
impl BackendDevice for Xe {}
