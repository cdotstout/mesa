// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::os::fd::AsFd;
use std::sync::Arc;

use magma_gpu::util::OwnedDescriptor;
use zerocopy::IntoBytes;

use crate::defines::buffer_flags_get_map_info;
use crate::defines::MagmaAddressSpaceProperties;
use crate::defines::MagmaCreateBufferInfo;
use crate::defines::MagmaCreateQueueInfo;
use crate::defines::MagmaCreateSemaphoreInfo;
use crate::defines::MagmaImportHandleInfo;
use crate::defines::MagmaPhysicalDeviceInfo;
use crate::defines::MagmaQueueFamilyProperties;
use crate::defines::MagmaQueueFlags;
use crate::defines::MagmaQueuePriority;
use crate::defines::MagmaSyncType;
use crate::defines::MagmaTimestampProperties;
use crate::error::Error;
use crate::error::Result;
use crate::sys::linux::amdgpu::ioctl::amdgpu_info_dev;
use crate::sys::linux::amdgpu::ioctl::amdgpu_info_hw_ip;
use crate::sys::linux::amdgpu::memory::AmdGpuAddressSpace;
use crate::sys::linux::amdgpu::memory::AmdGpuBuffer;
use crate::sys::linux::amdgpu::queue::AmdGpuQueue;
use crate::sys::linux::bindings::amdgpu_bindings::AMDGPU_HW_IP_COMPUTE;
use crate::sys::linux::bindings::amdgpu_bindings::AMDGPU_HW_IP_GFX;
use crate::sys::linux::drm::DrmSemaphore;
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
use crate::vendor::amdgpu::MagmaAmdgpuDeviceInfo;
use crate::vendor::amdgpu::MAGMA_AMDGPU_QUERY_DEVICE_INFO;
use crate::traits::GenericSemaphore;

#[derive(Debug)]
pub struct AmdGpuPhysicalDevice {
    descriptor: LinuxDescriptor,
}

pub struct AmdGpu {
    physical_device: Arc<dyn BackendPhysicalDevice>,
}

impl AmdGpuPhysicalDevice {
    pub fn new(descriptor: OwnedDescriptor) -> AmdGpuPhysicalDevice {
        AmdGpuPhysicalDevice {
            descriptor: LinuxDescriptor::new(descriptor),
        }
    }
}

impl PlatformPhysicalDevice for AmdGpuPhysicalDevice {
    fn as_linux_descriptor(&self) -> Option<&LinuxDescriptor> {
        Some(&self.descriptor)
    }
}

impl AsVirtGpu for AmdGpuPhysicalDevice {}
impl BackendPhysicalDevice for AmdGpuPhysicalDevice {}

impl GenericPhysicalDevice for AmdGpuPhysicalDevice {
    fn create_device(
        self: Arc<AmdGpuPhysicalDevice>,
        _info: &MagmaPhysicalDeviceInfo,
    ) -> Result<Arc<dyn BackendDevice>> {
        Ok(Arc::new(AmdGpu::new(self)?))
    }

    fn get_queue_family_properties(&self) -> Result<Vec<MagmaQueueFamilyProperties>> {
        let mut queue_families: Vec<MagmaQueueFamilyProperties> = Vec::new();
        let fd = self.descriptor.as_fd();

        let gfx_hw_ip = amdgpu_info_hw_ip(fd, AMDGPU_HW_IP_GFX, 0)?;
        if gfx_hw_ip.available_rings > 0 {
            queue_families.push(MagmaQueueFamilyProperties::new(
                MagmaQueueFlags::Graphics | MagmaQueueFlags::Compute,
                1,
            ));
        }

        if let Ok(compute_hw_ip) = amdgpu_info_hw_ip(fd, AMDGPU_HW_IP_COMPUTE, 0) {
            if compute_hw_ip.available_rings > 0 {
                queue_families.push(MagmaQueueFamilyProperties::new(
                    MagmaQueueFlags::Compute,
                    compute_hw_ip.available_rings.count_ones(),
                ));
            }
        }

        if queue_families.is_empty() {
            return Err(Error::InternalError);
        }

        Ok(queue_families)
    }

    fn get_address_space_properties(&self) -> Result<MagmaAddressSpaceProperties> {
        let dev = amdgpu_info_dev(self.descriptor.as_fd())?;
        let va_start = dev.virtual_address_offset;
        let va_size = dev
            .virtual_address_max
            .saturating_sub(dev.virtual_address_offset);
        let mem_alignment = dev.virtual_address_alignment as u64;
        Ok(MagmaAddressSpaceProperties::new(
            va_start,
            va_size,
            mem_alignment,
        ))
    }

    fn get_timestamp_properties(&self) -> Result<MagmaTimestampProperties> {
        let dev = amdgpu_info_dev(self.descriptor.as_fd())?;
        Ok(MagmaTimestampProperties::new(
            dev.gpu_counter_freq as u64 * 1000,
        ))
    }

    fn query_vendor(
        &self,
        query_id: u64,
        buffer: Option<&mut [u8]>,
    ) -> Result<MagmaVendorQueryResult> {
        match query_id {
            MAGMA_AMDGPU_QUERY_DEVICE_INFO => {
                let dev = amdgpu_info_dev(self.descriptor.as_fd())?;
                let info = MagmaAmdgpuDeviceInfo {
                    max_engine_clock: dev.max_engine_clock,
                    max_memory_clock: dev.max_memory_clock,
                    ids_flags: dev.ids_flags,
                    chip_rev: dev.chip_rev,
                    external_rev: dev.external_rev,
                    family: dev.family,
                    num_shader_engines: dev.num_shader_engines,
                    num_shader_arrays_per_engine: dev.num_shader_arrays_per_engine,
                    cu_active_number: dev.cu_active_number,
                    enabled_rb_pipes_mask: dev.enabled_rb_pipes_mask,
                    num_rb_pipes: dev.num_rb_pipes,
                    vram_type: dev.vram_type,
                    vram_bit_width: dev.vram_bit_width,
                    wave_front_size: dev.wave_front_size,
                    num_shader_visible_vgprs: dev.num_shader_visible_vgprs,
                    num_cu_per_sh: dev.num_cu_per_sh,
                    num_tcc_blocks: dev.num_tcc_blocks,
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

impl AmdGpu {
    pub fn new(physical_device: Arc<dyn BackendPhysicalDevice>) -> Result<AmdGpu> {
        Ok(AmdGpu { physical_device })
    }
}

impl GenericDevice for AmdGpu {
    fn create_address_space(self: Arc<AmdGpu>) -> Result<Arc<dyn BackendAddressSpace>> {
        Ok(Arc::new(AmdGpuAddressSpace {
            physical_device: self.physical_device.clone(),
        }))
    }

    fn create_queue(
        self: Arc<AmdGpu>,
        address_space: &Arc<dyn BackendAddressSpace>,
        info: &MagmaCreateQueueInfo,
    ) -> Result<Arc<dyn BackendQueue>> {
        let priority = match info.priority {
            MagmaQueuePriority::Low => -512,
            MagmaQueuePriority::Medium => 0,
            MagmaQueuePriority::High => 512,
            MagmaQueuePriority::Realtime => 1023,
        };
        let queue = AmdGpuQueue::new(
            self.physical_device.clone(),
            address_space.clone(),
            priority,
        )?;
        Ok(Arc::new(queue))
    }

    fn create_buffer(
        self: Arc<AmdGpu>,
        create_info: &MagmaCreateBufferInfo,
    ) -> Result<Arc<dyn BackendBuffer>> {
        let buf = AmdGpuBuffer::new(self.physical_device.clone(), create_info)?;
        let buf: Arc<dyn BackendBuffer> = Arc::new(buf);
        self.physical_device.register_buffer(&buf);
        Ok(buf)
    }

    fn import(self: Arc<AmdGpu>, info: MagmaImportHandleInfo) -> Result<Arc<dyn BackendBuffer>> {
        let map_info = buffer_flags_get_map_info(info.common_flags);
        let gem_handle = self.physical_device.import(info.handle)?;
        if let Some(existing) = self.physical_device.find_buffer(gem_handle) {
            return Ok(existing);
        }
        let buf = AmdGpuBuffer::from_existing(
            self.physical_device.clone(),
            gem_handle,
            info.size.try_into()?,
            map_info,
        )?;
        let buf: Arc<dyn BackendBuffer> = Arc::new(buf);
        self.physical_device.register_buffer(&buf);
        Ok(buf)
    }

    fn create_semaphore(
        self: Arc<AmdGpu>,
        info: &MagmaCreateSemaphoreInfo,
    ) -> Result<Arc<dyn BackendSemaphore>> {
        let semaphore = DrmSemaphore::new_from_info(self.physical_device.clone(), info)?;
        Ok(Arc::new(semaphore))
    }

    fn import_semaphore(
        self: Arc<AmdGpu>,
        info: MagmaImportHandleInfo,
    ) -> Result<Arc<dyn BackendSemaphore>> {
        let semaphore = DrmSemaphore::new(self.physical_device.clone(), MagmaSyncType::Binary)?;
        semaphore.import(info.handle)?;
        Ok(Arc::new(semaphore))
    }
}

impl BackendDevice for AmdGpu {}
impl PlatformDevice for AmdGpu {}
