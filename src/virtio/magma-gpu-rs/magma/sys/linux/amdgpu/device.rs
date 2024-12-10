// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::os::fd::AsFd;
use std::sync::Arc;

use magma_gpu::util::OwnedDescriptor;

use crate::defines::MagmaAddressSpaceProperties;
use crate::defines::MagmaAmdgpuDeviceInfo;
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
use crate::defines::MAGMA_HEAP_CPU_VISIBLE_BIT;
use crate::defines::MAGMA_HEAP_DEVICE_LOCAL_BIT;
use crate::defines::MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT;
use crate::defines::MAGMA_MEMORY_PROPERTY_HOST_CACHED_BIT;
use crate::defines::MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT;
use crate::defines::MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT;
use crate::error::Error;
use crate::error::Result;
use crate::sys::linux::amdgpu::ioctl::amdgpu_info_dev;
use crate::sys::linux::amdgpu::ioctl::amdgpu_info_gtt_usage;
use crate::sys::linux::amdgpu::ioctl::amdgpu_info_hw_ip;
use crate::sys::linux::amdgpu::ioctl::amdgpu_info_memory;
use crate::sys::linux::amdgpu::ioctl::amdgpu_info_vis_vram_usage;
use crate::sys::linux::amdgpu::ioctl::amdgpu_info_vram_gtt;
use crate::sys::linux::amdgpu::ioctl::amdgpu_info_vram_usage;
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
use crate::traits::GenericSemaphore;

#[derive(Debug)]
pub struct AmdGpuPhysicalDevice {
    descriptor: LinuxDescriptor,
}

pub struct AmdGpu {
    physical_device: Arc<dyn BackendPhysicalDevice>,
    mem_types: Vec<MagmaMemoryType>,
    mem_heaps: Vec<MagmaHeap>,
}

impl AmdGpuPhysicalDevice {
    pub fn new(descriptor: OwnedDescriptor) -> AmdGpuPhysicalDevice {
        AmdGpuPhysicalDevice {
            descriptor: LinuxDescriptor::new(descriptor),
        }
    }

    fn query_memory(&self) -> Result<(Vec<MagmaMemoryType>, Vec<MagmaHeap>)> {
        let memory_info = amdgpu_info_memory(self.descriptor.as_fd())?;

        let mut heaps = Vec::new();
        let mut types = Vec::new();
        let mut heap_idx = 0u32;

        if memory_info.gtt.total_heap_size > 0 {
            heaps.push(MagmaHeap {
                heap_size: memory_info.gtt.total_heap_size,
                heap_flags: MAGMA_HEAP_CPU_VISIBLE_BIT,
            });
            types.push(MagmaMemoryType {
                property_flags: MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT
                    | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT,
                heap_idx,
            });
            types.push(MagmaMemoryType {
                property_flags: MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT
                    | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT
                    | MAGMA_MEMORY_PROPERTY_HOST_CACHED_BIT,
                heap_idx,
            });
            heap_idx += 1;
        }

        if memory_info.vram.total_heap_size > 0 {
            heaps.push(MagmaHeap {
                heap_size: memory_info.vram.total_heap_size,
                heap_flags: MAGMA_HEAP_DEVICE_LOCAL_BIT,
            });
            types.push(MagmaMemoryType {
                property_flags: MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT,
                heap_idx,
            });
            heap_idx += 1;
        }

        if memory_info.cpu_accessible_vram.total_heap_size > 0 {
            heaps.push(MagmaHeap {
                heap_size: memory_info.cpu_accessible_vram.total_heap_size,
                heap_flags: MAGMA_HEAP_DEVICE_LOCAL_BIT | MAGMA_HEAP_CPU_VISIBLE_BIT,
            });
            types.push(MagmaMemoryType {
                property_flags: MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
                    | MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT
                    | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT,
                heap_idx,
            });
        }

        Ok((types, heaps))
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

    fn get_memory_types(&self) -> Result<Vec<MagmaMemoryType>> {
        self.query_memory().map(|(types, _)| types)
    }

    fn get_memory_heaps(&self) -> Result<Vec<MagmaHeap>> {
        self.query_memory().map(|(_, heaps)| heaps)
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

    fn get_amdgpu_device_info(&self) -> Result<MagmaAmdgpuDeviceInfo> {
        let dev = amdgpu_info_dev(self.descriptor.as_fd())?;
        Ok(MagmaAmdgpuDeviceInfo::new(
            dev.max_engine_clock,
            dev.max_memory_clock,
            dev.ids_flags,
            dev.chip_rev,
            dev.external_rev,
            dev.family,
            dev.num_shader_engines,
            dev.num_shader_arrays_per_engine,
            dev.cu_active_number,
            dev.enabled_rb_pipes_mask,
            dev.num_rb_pipes,
            dev.vram_type,
            dev.vram_bit_width,
            dev.wave_front_size,
            dev.num_shader_visible_vgprs,
            dev.num_cu_per_sh,
            dev.num_tcc_blocks,
        ))
    }
}

impl AmdGpu {
    pub fn new(physical_device: Arc<dyn BackendPhysicalDevice>) -> Result<AmdGpu> {
        let mem_types = physical_device.get_memory_types()?;
        let mem_heaps = physical_device.get_memory_heaps()?;

        Ok(AmdGpu {
            physical_device,
            mem_types,
            mem_heaps,
        })
    }
}

impl GenericDevice for AmdGpu {
    fn get_memory_budget(&self, heap_idx: u32) -> Result<MagmaHeapBudget> {
        let heap = self
            .mem_heaps
            .get(heap_idx as usize)
            .ok_or(Error::InvalidArgs)?;
        let fd = self.physical_device.as_fd().unwrap();

        let vram_gtt = amdgpu_info_vram_gtt(fd)?;

        let (budget, usage) = if heap.is_device_local() && heap.is_cpu_visible() {
            (
                vram_gtt.vram_cpu_accessible_size,
                amdgpu_info_vis_vram_usage(fd)?,
            )
        } else if heap.is_device_local() {
            (vram_gtt.vram_size, amdgpu_info_vram_usage(fd)?)
        } else if heap.is_cpu_visible() {
            (vram_gtt.gtt_size, amdgpu_info_gtt_usage(fd)?)
        } else {
            return Err(Error::Unimplemented);
        };

        Ok(MagmaHeapBudget { budget, usage })
    }

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
        let queue = AmdGpuQueue::new(
            self.physical_device.clone(),
            address_space.clone(),
            info.priority as i32,
        )?;
        Ok(Arc::new(queue))
    }

    fn create_buffer(
        self: Arc<AmdGpu>,
        create_info: &MagmaCreateBufferInfo,
    ) -> Result<Arc<dyn BackendBuffer>> {
        let buf = AmdGpuBuffer::new(self.physical_device.clone(), create_info, &self.mem_types)?;
        let buf: Arc<dyn BackendBuffer> = Arc::new(buf);
        self.physical_device.register_buffer(&buf);
        Ok(buf)
    }

    fn import(self: Arc<AmdGpu>, info: MagmaImportHandleInfo) -> Result<Arc<dyn BackendBuffer>> {
        let map_info = self
            .mem_types
            .get(info.memory_type_idx as usize)
            .and_then(|mt| mt.get_map_info());
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
