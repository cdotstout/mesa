// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use crate::defines::MagmaAddressSpaceProperties;
use crate::defines::MagmaAdrenoDeviceInfo;
use crate::defines::MagmaAmdgpuDeviceInfo;
use crate::defines::MagmaArmDeviceInfo;
use crate::defines::MagmaCreateBufferInfo;
use crate::defines::MagmaCreateQueueInfo;
use crate::defines::MagmaCreateSemaphoreInfo;
use crate::defines::MagmaHeap;
use crate::defines::MagmaHeapBudget;
use crate::defines::MagmaImportHandleInfo;
use crate::defines::MagmaIntelDeviceInfo;
use crate::defines::MagmaMemoryType;
use crate::defines::MagmaPhysicalDeviceInfo;
use crate::defines::MagmaQueueFamilyProperties;
use crate::defines::MagmaTimestampProperties;
use crate::error::Result;
use crate::memory::AddressSpace;
use crate::memory::Buffer;
use crate::queue::Queue;
use crate::semaphore::Semaphore;
use crate::sys::platform::enumerate_devices as platform_enumerate_devices;
use crate::traits::BackendDevice;
use crate::traits::BackendPhysicalDevice;
use crate::virtgpu_shared::kumquat::enumerate_devices as kumquat_enumerate_devices;

const VIRTGPU_KUMQUAT_ENABLED: &str = "VIRTGPU_KUMQUAT";

#[repr(C)]
#[derive(Clone)]
pub struct PhysicalDevice {
    physical_device: Arc<dyn BackendPhysicalDevice>,
    info: MagmaPhysicalDeviceInfo,
}

#[derive(Clone)]
pub struct Device {
    device: Arc<dyn BackendDevice>,
}

pub fn enumerate_devices() -> Result<Vec<PhysicalDevice>> {
    match std::env::var(VIRTGPU_KUMQUAT_ENABLED) {
        Ok(_) => kumquat_enumerate_devices(),
        Err(_) => platform_enumerate_devices(),
    }
}

impl PhysicalDevice {
    pub fn new(
        physical_device: Arc<dyn BackendPhysicalDevice>,
        info: MagmaPhysicalDeviceInfo,
    ) -> PhysicalDevice {
        PhysicalDevice {
            physical_device,
            info,
        }
    }

    pub fn info(&self) -> &MagmaPhysicalDeviceInfo {
        &self.info
    }

    pub fn get_memory_types(&self) -> Result<Vec<MagmaMemoryType>> {
        self.physical_device.get_memory_types()
    }

    pub fn get_memory_heaps(&self) -> Result<Vec<MagmaHeap>> {
        self.physical_device.get_memory_heaps()
    }

    pub fn get_queue_family_properties(&self) -> Result<Vec<MagmaQueueFamilyProperties>> {
        self.physical_device.get_queue_family_properties()
    }

    pub fn get_address_space_properties(&self) -> Result<MagmaAddressSpaceProperties> {
        self.physical_device.get_address_space_properties()
    }

    pub fn get_timestamp_properties(&self) -> Result<MagmaTimestampProperties> {
        self.physical_device.get_timestamp_properties()
    }

    pub fn get_intel_device_info(&self) -> Result<MagmaIntelDeviceInfo> {
        self.physical_device.get_intel_device_info()
    }

    pub fn get_adreno_device_info(&self) -> Result<MagmaAdrenoDeviceInfo> {
        self.physical_device.get_adreno_device_info()
    }

    pub fn get_arm_device_info(&self) -> Result<MagmaArmDeviceInfo> {
        self.physical_device.get_arm_device_info()
    }

    pub fn get_amdgpu_device_info(&self) -> Result<MagmaAmdgpuDeviceInfo> {
        self.physical_device.get_amdgpu_device_info()
    }

    pub fn create_device(&self) -> Result<Device> {
        let device = self.physical_device.clone().create_device(&self.info)?;
        Ok(Device { device })
    }
}

impl Device {
    pub fn get_memory_budget(&self, heap_idx: u32) -> Result<MagmaHeapBudget> {
        self.device.get_memory_budget(heap_idx)
    }

    pub fn create_address_space(&self) -> Result<AddressSpace> {
        let address_space = self.device.clone().create_address_space()?;
        Ok(AddressSpace::new(address_space))
    }

    pub fn create_queue(
        &self,
        address_space: &AddressSpace,
        info: &MagmaCreateQueueInfo,
    ) -> Result<Queue> {
        let queue = self
            .device
            .clone()
            .create_queue(address_space.inner(), info)?;
        Ok(Queue::new(queue))
    }

    pub fn create_buffer(&self, create_info: &MagmaCreateBufferInfo) -> Result<Buffer> {
        let buffer = self.device.clone().create_buffer(create_info)?;
        Ok(Buffer::new(buffer))
    }

    // FIXME: we probably want to import with a memory type
    pub fn import(&self, info: MagmaImportHandleInfo) -> Result<Buffer> {
        let buffer = self.device.clone().import(info)?;
        Ok(Buffer::new(buffer))
    }

    pub fn create_semaphore(&self, info: &MagmaCreateSemaphoreInfo) -> Result<Semaphore> {
        let semaphore = self.device.clone().create_semaphore(info)?;
        Ok(Semaphore::new(semaphore))
    }

    pub fn import_semaphore(&self, info: MagmaImportHandleInfo) -> Result<Semaphore> {
        let semaphore = self.device.clone().import_semaphore(info)?;
        Ok(Semaphore::new(semaphore))
    }
}
