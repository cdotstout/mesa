// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::cmp::min;
use std::os::fd::AsFd;
use std::sync::Arc;

use magma_gpu::util::OwnedDescriptor;

use crate::defines::MagmaCreateBufferInfo;
use crate::defines::MagmaCreateQueueInfo;
use crate::defines::MagmaCreateSyncObjInfo;
use crate::defines::MagmaHeap;
use crate::defines::MagmaImportHandleInfo;
use crate::defines::MagmaMemoryType;
use crate::defines::MagmaPhysicalDeviceInfo;
use crate::defines::MagmaQueueFamilyProperties;
use crate::defines::MagmaQueueFlags;
use crate::defines::MagmaSyncType;
use crate::MagmaGpuMapFlags;
use crate::error::Error;
use crate::error::Result;
use crate::sys::linux::bindings::xe_bindings::__u64;
use crate::sys::linux::bindings::xe_bindings::drm_xe_query_config;
use crate::sys::linux::bindings::xe_bindings::DRM_XE_DEVICE_QUERY_CONFIG;
use crate::sys::linux::bindings::xe_bindings::DRM_XE_QUERY_CONFIG_MIN_ALIGNMENT;
use crate::sys::linux::bindings::xe_bindings::DRM_XE_QUERY_CONFIG_VA_BITS;
use crate::sys::linux::drm::DrmSyncObject;
use crate::sys::linux::xe::ioctl::determine_graphics_version;
use crate::sys::linux::xe::ioctl::xe_device_query;
use crate::sys::linux::xe::ioctl::xe_query_memory;
use crate::sys::linux::xe::ioctl::xe_query_memory_regions;
use crate::sys::linux::xe::memory::XeAddressSpace;
use crate::sys::linux::xe::memory::XeBuffer;
use crate::sys::linux::xe::queue::XeQueue;
use crate::sys::linux::PlatformDevice;
use crate::sys::linux::PlatformPhysicalDevice;
use crate::traits::AsVirtGpu;
use crate::traits::BackendAddressSpace;
use crate::traits::BackendBuffer;
use crate::traits::BackendDevice;
use crate::traits::BackendPhysicalDevice;
use crate::traits::BackendQueue;
use crate::traits::BackendSyncObject;
use crate::traits::GenericDevice;
use crate::traits::GenericPhysicalDevice;
use crate::traits::GenericSyncObject;

#[derive(Debug)]
pub struct XePhysicalDevice {
    descriptor: OwnedDescriptor,
}

pub struct Xe {
    physical_device: Arc<dyn BackendPhysicalDevice>,
    _gtt_size: u64,
    _mem_alignment: u64,
    mem_types: Vec<MagmaMemoryType>,
    mem_heaps: Vec<MagmaHeap>,
    sysmem_instance: u16,
    vram_instance: u16,
    address_space: Arc<dyn BackendAddressSpace>,
}

impl XePhysicalDevice {
    pub fn new(descriptor: OwnedDescriptor) -> XePhysicalDevice {
        XePhysicalDevice { descriptor }
    }
}

impl PlatformPhysicalDevice for XePhysicalDevice {
    fn as_descriptor(&self) -> Option<&OwnedDescriptor> {
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
}

impl Xe {
    pub fn new(
        physical_device: Arc<dyn BackendPhysicalDevice>,
        info: &MagmaPhysicalDeviceInfo,
    ) -> Result<Xe> {
        let fd = physical_device.as_fd().unwrap();
        let _graphics_version = determine_graphics_version(info.device_id)?;

        let query_config = xe_device_query::<
            drm_xe_query_config<[__u64]>,
            __u64,
            drm_xe_query_config<[__u64; 0]>,
        >(fd, DRM_XE_DEVICE_QUERY_CONFIG)?;

        let num_params = min(query_config.num_params as usize, query_config.info.len());
        let config = &query_config.info[..num_params];

        let gtt_size = 1u64 << config[DRM_XE_QUERY_CONFIG_VA_BITS as usize];
        let mem_alignment = config[DRM_XE_QUERY_CONFIG_MIN_ALIGNMENT as usize];

        let memory_info = xe_query_memory_regions(fd)?;
        let (mem_types, mem_heaps) = xe_query_memory(fd)?;

        let address_space = XeAddressSpace::new(physical_device.clone(), 0)?;

        Ok(Xe {
            physical_device,
            _gtt_size: gtt_size,
            _mem_alignment: mem_alignment,
            mem_types,
            mem_heaps,
            sysmem_instance: memory_info.sysmem_instance,
            vram_instance: memory_info.vram_instance,
            address_space: Arc::new(address_space),
        })
    }
}

impl GenericDevice for Xe {
    fn map_buffer_gpu(
        &self,
        buffer: &Arc<dyn BackendBuffer>,
        buffer_offset: u64,
        gpu_va: u64,
        size: u64,
        flags: MagmaGpuMapFlags,
    ) -> Result<()> {
        self.address_space.map_buffer_gpu(
            buffer,
            buffer_offset,
            gpu_va,
            size,
            flags,
        )
    }

    fn unmap_buffer_gpu(&self, gpu_va: u64, size: u64) -> Result<()> {
        self.address_space.unmap_buffer_gpu(gpu_va, size)
    }

    fn create_queue(
        self: Arc<Xe>,
        info: &MagmaCreateQueueInfo,
    ) -> Result<Arc<dyn BackendQueue>> {
        let address_space = self.address_space.clone();
        let queue = XeQueue::new(self.physical_device.clone(), &address_space, info)?;
        Ok(Arc::new(queue))
    }

    fn create_buffer(
        self: Arc<Xe>,
        create_info: &MagmaCreateBufferInfo,
    ) -> Result<Arc<dyn BackendBuffer>> {
        let buf = XeBuffer::new(
            self.physical_device.clone(),
            create_info,
            &self.mem_types,
            &self.mem_heaps,
            self.sysmem_instance,
            self.vram_instance,
        )?;
        Ok(Arc::new(buf))
    }

    fn import(self: Arc<Xe>, info: MagmaImportHandleInfo) -> Result<Arc<dyn BackendBuffer>> {
        let gem_handle = self.physical_device.import(info.handle)?;
        let memory_type = self
            .mem_types
            .get(info.memory_type_idx as usize)
            .ok_or(Error::InvalidArgs)?;
        let buf = XeBuffer::from_existing(
            self.physical_device.clone(),
            gem_handle,
            info.size.try_into()?,
            memory_type.get_map_info(),
        )?;
        Ok(Arc::new(buf))
    }

    fn create_sync_obj(
        self: Arc<Xe>,
        info: &MagmaCreateSyncObjInfo,
    ) -> Result<Arc<dyn BackendSyncObject>> {
        let sync_obj = DrmSyncObject::new_from_info(self.physical_device.clone(), info)?;
        Ok(Arc::new(sync_obj))
    }

    fn import_sync_obj(
        self: Arc<Xe>,
        info: MagmaImportHandleInfo,
    ) -> Result<Arc<dyn BackendSyncObject>> {
        let sync_obj = DrmSyncObject::new(self.physical_device.clone(), MagmaSyncType::Binary)?;
        sync_obj.import(info.handle)?;
        Ok(Arc::new(sync_obj))
    }
}

impl PlatformDevice for Xe {}
impl BackendDevice for Xe {}
