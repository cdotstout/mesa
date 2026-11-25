// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::os::fd::AsFd;
use std::sync::Arc;

use log::error;

use magma_gpu::log_status;
use magma_gpu::util::Handle as MagmaGpuHandle;
use magma_gpu::util::MemoryMapping;
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
use crate::defines::MagmaTimestampProperties;
use crate::error::Error;
use crate::error::Result;
use crate::sys::linux::bindings::kgsl_bindings::*;
use crate::sys::linux::kgsl::ioctl;
use crate::sys::linux::kgsl::memory::KgslAddressSpace;
use crate::sys::linux::kgsl::memory::KgslBuffer;
use crate::sys::linux::kgsl::queue::KgslQueue;
use crate::sys::linux::kgsl::semaphore::KgslSemaphore;
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
use crate::vendor::adreno::MagmaAdrenoDeviceInfo;
use crate::vendor::adreno::MAGMA_ADRENO_QUERY_DEVICE_INFO;

#[derive(Debug)]
#[allow(dead_code)]
pub struct KgslPhysicalDevice {
    descriptor: LinuxDescriptor,
    chip_id: u64,
    gpu_id: u32,
    gmem_size: u64,
}

pub struct Kgsl {
    physical_device: Arc<dyn BackendPhysicalDevice>,
}

#[allow(dead_code)]
impl KgslPhysicalDevice {
    pub fn new(descriptor: OwnedDescriptor) -> KgslPhysicalDevice {
        let (chip_id, gpu_id, gmem_size) = if let Ok(info) = ioctl::get_devinfo(descriptor.as_fd())
        {
            let cid = info.chip_id as u64;
            let gid = if info.gpu_id != 0 {
                info.gpu_id
            } else {
                (((cid >> 24) & 0xff) * 100 + ((cid >> 16) & 0xff) * 10 + ((cid >> 8) & 0xff))
                    as u32
            };
            (cid, gid, info.gmem_sizebytes)
        } else {
            (0, 0, 0)
        };

        KgslPhysicalDevice {
            descriptor: LinuxDescriptor::new(descriptor),
            chip_id,
            gpu_id,
            gmem_size: gmem_size as u64,
        }
    }

    pub fn chip_id(&self) -> u64 {
        self.chip_id
    }

    pub fn gpu_id(&self) -> u32 {
        self.gpu_id
    }

    pub fn gmem_size(&self) -> u64 {
        self.gmem_size
    }
}

impl PlatformPhysicalDevice for KgslPhysicalDevice {
    fn as_linux_descriptor(&self) -> Option<&LinuxDescriptor> {
        Some(&self.descriptor)
    }

    fn cpu_map(&self, offset: u64, size: usize) -> Result<MemoryMapping> {
        let desc = self.as_descriptor().ok_or(Error::Unimplemented)?;
        Ok(MemoryMapping::from_offset(desc, offset, size)?)
    }

    fn export(&self, _gem_handle: u32) -> Result<MagmaGpuHandle> {
        Err(Error::Unimplemented)
    }

    fn import(&self, handle: MagmaGpuHandle) -> Result<u32> {
        let fd = self.as_fd().unwrap();
        ioctl::gpuobj_import_dmabuf(fd, &handle)
    }

    fn close(&self, gem_handle: u32) {
        self.unregister_buffer(gem_handle);
        let result = ioctl::gpumem_free_id(self.as_fd().unwrap(), gem_handle);
        log_status!(result);
    }
}

impl AsVirtGpu for KgslPhysicalDevice {}
impl BackendPhysicalDevice for KgslPhysicalDevice {}

impl GenericPhysicalDevice for KgslPhysicalDevice {
    fn create_device(
        self: Arc<KgslPhysicalDevice>,
        _info: &MagmaPhysicalDeviceInfo,
    ) -> Result<Arc<dyn BackendDevice>> {
        Ok(Arc::new(Kgsl::new(self)?))
    }

    fn get_queue_family_properties(&self) -> Result<Vec<MagmaQueueFamilyProperties>> {
        Ok(vec![MagmaQueueFamilyProperties::new(
            MagmaQueueFlags::Graphics | MagmaQueueFlags::Compute,
            1,
        )])
    }

    fn get_address_space_properties(&self) -> Result<MagmaAddressSpaceProperties> {
        Ok(MagmaAddressSpaceProperties::new(0, 1u64 << 48, 4096))
    }

    fn get_timestamp_properties(&self) -> Result<MagmaTimestampProperties> {
        Ok(MagmaTimestampProperties::new(19_200_000))
    }

    fn query_vendor(
        &self,
        query_id: u64,
        buffer: Option<&mut [u8]>,
    ) -> Result<MagmaVendorQueryResult> {
        match query_id {
            MAGMA_ADRENO_QUERY_DEVICE_INFO => {
                let fd = self.descriptor.as_fd();
                let dev_info = ioctl::get_devinfo(fd).unwrap_or_default();
                let ubwc_mode = ioctl::get_u32_prop(fd, KGSL_PROP_UBWC_MODE).unwrap_or(0);
                let highest_bank_bit =
                    ioctl::get_u32_prop(fd, KGSL_PROP_HIGHEST_BANK_BIT).unwrap_or(0);
                let info = MagmaAdrenoDeviceInfo {
                    chip_id: self.chip_id,
                    gmem_size: self.gmem_size,
                    gmem_base: dev_info.gmem_gpubaseaddr,
                    gpu_id: self.gpu_id,
                    highest_bank_bit,
                    ubwc_swizzle: ubwc_mode,
                    macrotile_mode: 0,
                    raytracing: 0,
                    flags: 0,
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

impl Kgsl {
    pub fn new(physical_device: Arc<dyn BackendPhysicalDevice>) -> Result<Kgsl> {
        Ok(Kgsl { physical_device })
    }
}

impl GenericDevice for Kgsl {
    fn create_address_space(self: Arc<Kgsl>) -> Result<Arc<dyn BackendAddressSpace>> {
        Ok(Arc::new(KgslAddressSpace::new(
            self.physical_device.clone(),
        )?))
    }

    fn create_queue(
        self: Arc<Kgsl>,
        address_space: &Arc<dyn BackendAddressSpace>,
        _info: &MagmaCreateQueueInfo,
    ) -> Result<Arc<dyn BackendQueue>> {
        Ok(Arc::new(KgslQueue::new(
            self.physical_device.clone(),
            address_space.clone(),
        )?))
    }

    fn create_buffer(
        self: Arc<Kgsl>,
        create_info: &MagmaCreateBufferInfo,
    ) -> Result<Arc<dyn BackendBuffer>> {
        let buf = KgslBuffer::new(self.physical_device.clone(), create_info)?;
        let buf: Arc<dyn BackendBuffer> = Arc::new(buf);
        self.physical_device.register_buffer(&buf);
        Ok(buf)
    }

    fn import(self: Arc<Kgsl>, info: MagmaImportHandleInfo) -> Result<Arc<dyn BackendBuffer>> {
        let id = self.physical_device.import(info.handle)?;
        if let Some(existing) = self.physical_device.find_buffer(id) {
            return Ok(existing);
        }
        let buf = KgslBuffer::from_existing(
            self.physical_device.clone(),
            id,
            info.size.try_into()?,
            buffer_flags_get_map_info(info.common_flags),
        )?;
        let buf: Arc<dyn BackendBuffer> = Arc::new(buf);
        self.physical_device.register_buffer(&buf);
        Ok(buf)
    }

    fn create_semaphore(
        self: Arc<Kgsl>,
        _info: &MagmaCreateSemaphoreInfo,
    ) -> Result<Arc<dyn BackendSemaphore>> {
        Ok(Arc::new(KgslSemaphore::new(self.physical_device.clone())?))
    }

    fn import_semaphore(
        self: Arc<Kgsl>,
        info: MagmaImportHandleInfo,
    ) -> Result<Arc<dyn BackendSemaphore>> {
        Ok(Arc::new(KgslSemaphore::from_import(
            self.physical_device.clone(),
            &info,
        )?))
    }
}

impl PlatformDevice for Kgsl {}
impl BackendDevice for Kgsl {}
