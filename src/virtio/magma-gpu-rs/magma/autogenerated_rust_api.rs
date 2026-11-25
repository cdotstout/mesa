// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT
//
// Generated via:
//   https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/magma/gorgonzola
//
// Submit patches, do not hand-edit.

use crate::protocol::*;

impl MagmaHeap {
    pub fn new(heap_size: u64, heap_flags: u64) -> MagmaHeap {
        MagmaHeap {
            heap_size,
            heap_flags,
        }
    }
}

impl MagmaPciBusInfo {
    pub fn new(
        domain: u16,
        subvendor_id: u16,
        subdevice_id: u16,
        revision_id: u8,
        bus: u8,
        device: u8,
        function: u8,
    ) -> MagmaPciBusInfo {
        MagmaPciBusInfo {
            domain,
            subvendor_id,
            subdevice_id,
            revision_id,
            bus,
            device,
            function,
            ..Default::default()
        }
    }
}

impl MagmaCreateQueueInfo {
    pub fn new(
        queue_family_idx: u32,
        priority: MagmaQueuePriority,
        flags: u32,
    ) -> MagmaCreateQueueInfo {
        MagmaCreateQueueInfo {
            queue_family_idx,
            priority,
            flags,
            ..Default::default()
        }
    }
}

impl MagmaQueueFamilyProperties {
    pub fn new(queue_flags: MagmaQueueFlags, queue_count: u32) -> MagmaQueueFamilyProperties {
        MagmaQueueFamilyProperties {
            queue_flags,
            queue_count,
        }
    }
}

impl MagmaCreateBufferInfo {
    pub fn new(
        alignment: u32,
        common_flags: u32,
        vendor_flags: u32,
        size: u64,
    ) -> MagmaCreateBufferInfo {
        MagmaCreateBufferInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::CreateBufferInfo as u32,
                size: std::mem::size_of::<MagmaCreateBufferInfo>() as u32,
                p_next: 0,
            },
            alignment,
            common_flags,
            vendor_flags,
            size,
            ..Default::default()
        }
    }

    pub fn placement_info(&self) -> Option<&MagmaBufferPlacementInfo> {
        self.get_extension::<MagmaBufferPlacementInfo>()
    }

    pub fn placement_info_mut(&mut self) -> Option<&mut MagmaBufferPlacementInfo> {
        unsafe { self.get_extension_mut::<MagmaBufferPlacementInfo>() }
    }
}

impl MagmaBufferPlacementInfo {
    pub fn new(heap_mask: u32) -> MagmaBufferPlacementInfo {
        MagmaBufferPlacementInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::BufferPlacementInfo as u32,
                size: std::mem::size_of::<MagmaBufferPlacementInfo>() as u32,
                p_next: 0,
            },
            heap_mask,
            ..Default::default()
        }
    }
}

impl MagmaDeviceCreateInfo {
    pub fn new(flags: u32) -> MagmaDeviceCreateInfo {
        MagmaDeviceCreateInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::DeviceCreateInfo as u32,
                size: std::mem::size_of::<MagmaDeviceCreateInfo>() as u32,
                p_next: 0,
            },
            flags,
            ..Default::default()
        }
    }
}

impl MagmaWireSubmitSyncInfo {
    pub fn new(
        num_wait_semaphores: u32,
        num_signal_semaphores: u32,
        wait_semaphores: Vec<u32>,
        signal_semaphores: Vec<u32>,
        wait_points: Vec<u64>,
        signal_points: Vec<u64>,
    ) -> MagmaWireSubmitSyncInfo {
        MagmaWireSubmitSyncInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitSyncInfo as u32,
                size: std::mem::size_of::<MagmaWireSubmitSyncInfo>() as u32,
                p_next: 0,
            },
            num_wait_semaphores,
            num_signal_semaphores,
            wait_semaphores,
            signal_semaphores,
            wait_points,
            signal_points,
        }
    }
}

impl MagmaWireSubmitInfo {
    pub fn new(flags: u32, sync_info: MagmaWireSubmitSyncInfo) -> MagmaWireSubmitInfo {
        MagmaWireSubmitInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitInfo as u32,
                size: std::mem::size_of::<MagmaWireSubmitInfo>() as u32,
                p_next: 0,
            },
            flags,
            sync_info,
            ..Default::default()
        }
    }

    pub fn address_space_info(&self) -> Option<&MagmaSubmitAddressSpaceInfo> {
        self.get_extension::<MagmaSubmitAddressSpaceInfo>()
    }

    pub fn address_space_info_mut(&mut self) -> Option<&mut MagmaSubmitAddressSpaceInfo> {
        unsafe { self.get_extension_mut::<MagmaSubmitAddressSpaceInfo>() }
    }

    pub fn unmap_buffer_info(&self) -> Option<&MagmaSubmitUnmapBufferInfo> {
        self.get_extension::<MagmaSubmitUnmapBufferInfo>()
    }

    pub fn unmap_buffer_info_mut(&mut self) -> Option<&mut MagmaSubmitUnmapBufferInfo> {
        unsafe { self.get_extension_mut::<MagmaSubmitUnmapBufferInfo>() }
    }

    pub fn virtio_seqno_info(&self) -> Option<&MagmaVirtioSeqnoInfo> {
        self.get_extension::<MagmaVirtioSeqnoInfo>()
    }

    pub fn virtio_seqno_info_mut(&mut self) -> Option<&mut MagmaVirtioSeqnoInfo> {
        unsafe { self.get_extension_mut::<MagmaVirtioSeqnoInfo>() }
    }

    pub fn buffer_info(&self) -> Option<&MagmaWireSubmitBufferInfo> {
        self.get_extension::<MagmaWireSubmitBufferInfo>()
    }

    pub fn buffer_info_mut(&mut self) -> Option<&mut MagmaWireSubmitBufferInfo> {
        unsafe { self.get_extension_mut::<MagmaWireSubmitBufferInfo>() }
    }

    pub fn map_buffer_info(&self) -> Option<&MagmaWireSubmitMapBufferInfo> {
        self.get_extension::<MagmaWireSubmitMapBufferInfo>()
    }

    pub fn map_buffer_info_mut(&mut self) -> Option<&mut MagmaWireSubmitMapBufferInfo> {
        unsafe { self.get_extension_mut::<MagmaWireSubmitMapBufferInfo>() }
    }

    pub fn resources_info(&self) -> Option<&MagmaWireSubmitResourcesInfo> {
        self.get_extension::<MagmaWireSubmitResourcesInfo>()
    }

    pub fn resources_info_mut(&mut self) -> Option<&mut MagmaWireSubmitResourcesInfo> {
        unsafe { self.get_extension_mut::<MagmaWireSubmitResourcesInfo>() }
    }
}

impl MagmaSubmitAddressSpaceInfo {
    pub fn new(command_va: u64, length: u64) -> MagmaSubmitAddressSpaceInfo {
        MagmaSubmitAddressSpaceInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitAddressSpaceInfo as u32,
                size: std::mem::size_of::<MagmaSubmitAddressSpaceInfo>() as u32,
                p_next: 0,
            },
            command_va,
            length,
        }
    }
}

impl MagmaWireSubmitBufferInfo {
    pub fn new(command_buffer: u32, start_offset: u64, length: u64) -> MagmaWireSubmitBufferInfo {
        MagmaWireSubmitBufferInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitBufferInfo as u32,
                size: std::mem::size_of::<MagmaWireSubmitBufferInfo>() as u32,
                p_next: 0,
            },
            command_buffer,
            start_offset,
            length,
            ..Default::default()
        }
    }
}

impl MagmaWireSubmitMapBufferInfo {
    pub fn new(
        buffer: u32,
        buffer_offset: u64,
        gpu_va: u64,
        size: u64,
        flags: MagmaGpuMapFlags,
    ) -> MagmaWireSubmitMapBufferInfo {
        MagmaWireSubmitMapBufferInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitMapBufferInfo as u32,
                size: std::mem::size_of::<MagmaWireSubmitMapBufferInfo>() as u32,
                p_next: 0,
            },
            buffer,
            buffer_offset,
            gpu_va,
            size,
            flags,
            ..Default::default()
        }
    }
}

impl MagmaSubmitUnmapBufferInfo {
    pub fn new(gpu_va: u64, size: u64) -> MagmaSubmitUnmapBufferInfo {
        MagmaSubmitUnmapBufferInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitUnmapBufferInfo as u32,
                size: std::mem::size_of::<MagmaSubmitUnmapBufferInfo>() as u32,
                p_next: 0,
            },
            gpu_va,
            size,
        }
    }
}

impl MagmaWireSubmitResourcesInfo {
    pub fn new(num_resources: u32, buffers: Vec<u32>) -> MagmaWireSubmitResourcesInfo {
        MagmaWireSubmitResourcesInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SubmitResourcesInfo as u32,
                size: std::mem::size_of::<MagmaWireSubmitResourcesInfo>() as u32,
                p_next: 0,
            },
            num_resources,
            buffers,
            ..Default::default()
        }
    }
}

impl MagmaCreateSemaphoreInfo {
    pub fn new(
        semaphore_type: u32,
        use_flags: MagmaSemaphoreUseFlags,
        initial_point: u64,
    ) -> MagmaCreateSemaphoreInfo {
        MagmaCreateSemaphoreInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::CreateSemaphoreInfo as u32,
                size: std::mem::size_of::<MagmaCreateSemaphoreInfo>() as u32,
                p_next: 0,
            },
            semaphore_type,
            use_flags,
            initial_point,
        }
    }
}

impl MagmaSyncProperties {
    pub fn new(capabilities: u32, max_timeline_value_difference: u64) -> MagmaSyncProperties {
        MagmaSyncProperties {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::SyncProperties as u32,
                size: std::mem::size_of::<MagmaSyncProperties>() as u32,
                p_next: 0,
            },
            capabilities,
            max_timeline_value_difference,
            ..Default::default()
        }
    }
}

impl MagmaVirtCapabilities {
    pub fn new(
        capset_version: u32,
        num_physical_devices: u32,
        blob_preference: VirtioGpuBlobPreference,
        ring_buffer_len: u64,
    ) -> MagmaVirtCapabilities {
        MagmaVirtCapabilities {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::VirtCapabilities as u32,
                size: std::mem::size_of::<MagmaVirtCapabilities>() as u32,
                p_next: 0,
            },
            capset_version,
            num_physical_devices,
            blob_preference,
            ring_buffer_len,
            ..Default::default()
        }
    }

    pub fn physical_device_info(&self) -> Option<&MagmaPhysicalDeviceInfo> {
        self.get_extension::<MagmaPhysicalDeviceInfo>()
    }

    pub fn physical_device_info_mut(&mut self) -> Option<&mut MagmaPhysicalDeviceInfo> {
        unsafe { self.get_extension_mut::<MagmaPhysicalDeviceInfo>() }
    }
}

impl MagmaPhysicalDeviceInfo {
    pub fn new(
        bus_type: u32,
        vendor_id: MagmaVendorId,
        device_id: u16,
        flags: u32,
        pci_bus_info: MagmaPciBusInfo,
        queue_family_count: u32,
        queue_families: [MagmaQueueFamilyProperties; MAGMA_MAX_QUEUES],
    ) -> MagmaPhysicalDeviceInfo {
        MagmaPhysicalDeviceInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::PhysicalDeviceInfo as u32,
                size: std::mem::size_of::<MagmaPhysicalDeviceInfo>() as u32,
                p_next: 0,
            },
            bus_type,
            vendor_id,
            device_id,
            flags,
            pci_bus_info,
            queue_family_count,
            queue_families,
        }
    }

    pub fn address_space_properties(&self) -> Option<&MagmaAddressSpaceProperties> {
        self.get_extension::<MagmaAddressSpaceProperties>()
    }

    pub fn address_space_properties_mut(&mut self) -> Option<&mut MagmaAddressSpaceProperties> {
        unsafe { self.get_extension_mut::<MagmaAddressSpaceProperties>() }
    }

    pub fn timestamp_properties(&self) -> Option<&MagmaTimestampProperties> {
        self.get_extension::<MagmaTimestampProperties>()
    }

    pub fn timestamp_properties_mut(&mut self) -> Option<&mut MagmaTimestampProperties> {
        unsafe { self.get_extension_mut::<MagmaTimestampProperties>() }
    }
}

impl MagmaAddressSpaceProperties {
    pub fn new(va_start: u64, va_size: u64, mem_alignment: u64) -> MagmaAddressSpaceProperties {
        MagmaAddressSpaceProperties {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::AddressSpaceProperties as u32,
                size: std::mem::size_of::<MagmaAddressSpaceProperties>() as u32,
                p_next: 0,
            },
            va_start,
            va_size,
            mem_alignment,
        }
    }
}

impl MagmaTimestampProperties {
    pub fn new(timestamp_frequency: u64) -> MagmaTimestampProperties {
        MagmaTimestampProperties {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::TimestampProperties as u32,
                size: std::mem::size_of::<MagmaTimestampProperties>() as u32,
                p_next: 0,
            },
            timestamp_frequency,
        }
    }
}

impl MagmaMemoryHeapsProperties {
    pub fn new(
        heap_count: u32,
        heaps: [MagmaHeap; MAGMA_MAX_MEMORY_HEAPS],
    ) -> MagmaMemoryHeapsProperties {
        MagmaMemoryHeapsProperties {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::MemoryHeapsProperties as u32,
                size: std::mem::size_of::<MagmaMemoryHeapsProperties>() as u32,
                p_next: 0,
            },
            heap_count,
            heaps,
            ..Default::default()
        }
    }
}

impl MagmaVirtioSeqnoInfo {
    pub fn new(ring_idx: u32, wait_seqno: u32) -> MagmaVirtioSeqnoInfo {
        MagmaVirtioSeqnoInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::VirtioSeqnoInfo as u32,
                size: std::mem::size_of::<MagmaVirtioSeqnoInfo>() as u32,
                p_next: 0,
            },
            ring_idx,
            wait_seqno,
        }
    }
}
