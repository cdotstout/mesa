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

impl MagmaHeapBudget {
    pub fn new(budget: u64, usage: u64) -> MagmaHeapBudget {
        MagmaHeapBudget { budget, usage }
    }
}

impl MagmaMemoryType {
    pub fn new(property_flags: u32, heap_idx: u32) -> MagmaMemoryType {
        MagmaMemoryType {
            property_flags,
            heap_idx,
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
    pub fn new(queue_family_idx: u32, priority: u32, flags: u32) -> MagmaCreateQueueInfo {
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
        memory_type_idx: u32,
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
            memory_type_idx,
            alignment,
            common_flags,
            vendor_flags,
            size,
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

impl MagmaMemoryProperties {
    pub fn new(
        memory_type_count: u32,
        memory_heap_count: u32,
        memory_types: [MagmaMemoryType; MAGMA_MAX_MEMORY_TYPES],
        memory_heaps: [MagmaHeap; MAGMA_MAX_MEMORY_HEAPS],
    ) -> MagmaMemoryProperties {
        MagmaMemoryProperties {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::MemoryProperties as u32,
                size: std::mem::size_of::<MagmaMemoryProperties>() as u32,
                p_next: 0,
            },
            memory_type_count,
            memory_heap_count,
            memory_types,
            memory_heaps,
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

    pub fn intel_device_info(&self) -> Option<&MagmaIntelDeviceInfo> {
        self.get_extension::<MagmaIntelDeviceInfo>()
    }

    pub fn intel_device_info_mut(&mut self) -> Option<&mut MagmaIntelDeviceInfo> {
        unsafe { self.get_extension_mut::<MagmaIntelDeviceInfo>() }
    }

    pub fn adreno_device_info(&self) -> Option<&MagmaAdrenoDeviceInfo> {
        self.get_extension::<MagmaAdrenoDeviceInfo>()
    }

    pub fn adreno_device_info_mut(&mut self) -> Option<&mut MagmaAdrenoDeviceInfo> {
        unsafe { self.get_extension_mut::<MagmaAdrenoDeviceInfo>() }
    }

    pub fn arm_device_info(&self) -> Option<&MagmaArmDeviceInfo> {
        self.get_extension::<MagmaArmDeviceInfo>()
    }

    pub fn arm_device_info_mut(&mut self) -> Option<&mut MagmaArmDeviceInfo> {
        unsafe { self.get_extension_mut::<MagmaArmDeviceInfo>() }
    }

    pub fn amdgpu_device_info(&self) -> Option<&MagmaAmdgpuDeviceInfo> {
        self.get_extension::<MagmaAmdgpuDeviceInfo>()
    }

    pub fn amdgpu_device_info_mut(&mut self) -> Option<&mut MagmaAmdgpuDeviceInfo> {
        unsafe { self.get_extension_mut::<MagmaAmdgpuDeviceInfo>() }
    }

    pub fn memory_type(&self) -> Option<&MagmaPhysicalDeviceMemoryType> {
        self.get_extension::<MagmaPhysicalDeviceMemoryType>()
    }

    pub fn memory_type_mut(&mut self) -> Option<&mut MagmaPhysicalDeviceMemoryType> {
        unsafe { self.get_extension_mut::<MagmaPhysicalDeviceMemoryType>() }
    }

    pub fn memory_heap(&self) -> Option<&MagmaPhysicalDeviceMemoryHeap> {
        self.get_extension::<MagmaPhysicalDeviceMemoryHeap>()
    }

    pub fn memory_heap_mut(&mut self) -> Option<&mut MagmaPhysicalDeviceMemoryHeap> {
        unsafe { self.get_extension_mut::<MagmaPhysicalDeviceMemoryHeap>() }
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

impl MagmaIntelDeviceInfo {
    pub fn new(
        hw_config_count: u32,
        l3_banks: u32,
        geo_dss_mask: u64,
        eu_per_dss_mask: u64,
        geo_dss_num_bytes: u32,
        ip_ver_major: u32,
        ip_ver_minor: u32,
        ip_ver_rev: u32,
        config_flags: MagmaIntelConfigFlag,
        hw_config_data: [u32; MAGMA_MAX_HW_CONFIG_DWORDS],
    ) -> MagmaIntelDeviceInfo {
        MagmaIntelDeviceInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::IntelDeviceInfo as u32,
                size: std::mem::size_of::<MagmaIntelDeviceInfo>() as u32,
                p_next: 0,
            },
            hw_config_count,
            l3_banks,
            geo_dss_mask,
            eu_per_dss_mask,
            geo_dss_num_bytes,
            ip_ver_major,
            ip_ver_minor,
            ip_ver_rev,
            config_flags,
            hw_config_data,
            ..Default::default()
        }
    }
}

impl MagmaAdrenoDeviceInfo {
    pub fn new(
        chip_id: u64,
        gmem_size: u64,
        gmem_base: u64,
        gpu_id: u32,
        highest_bank_bit: u32,
        ubwc_swizzle: u32,
        macrotile_mode: u32,
        raytracing: u32,
        flags: u32,
    ) -> MagmaAdrenoDeviceInfo {
        MagmaAdrenoDeviceInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::AdrenoDeviceInfo as u32,
                size: std::mem::size_of::<MagmaAdrenoDeviceInfo>() as u32,
                p_next: 0,
            },
            chip_id,
            gmem_size,
            gmem_base,
            gpu_id,
            highest_bank_bit,
            ubwc_swizzle,
            macrotile_mode,
            raytracing,
            flags,
        }
    }
}

impl MagmaArmDeviceInfo {
    pub fn new(
        shader_present: u64,
        tiler_present: u64,
        l2_present: u64,
        gpu_id: u32,
        gpu_rev: u32,
        csf_id: u32,
        l2_features: u32,
        tiler_features: u32,
        mem_features: u32,
        mmu_features: u32,
        thread_features: u32,
        max_threads: u32,
        thread_max_workgroup_size: u32,
        thread_max_barrier_size: u32,
        coherency_features: u32,
        selected_coherency: u32,
        as_present: u32,
        csg_slot_count: u32,
        cs_slot_count: u32,
        cs_reg_count: u32,
        scoreboard_slot_count: u32,
    ) -> MagmaArmDeviceInfo {
        MagmaArmDeviceInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::ArmDeviceInfo as u32,
                size: std::mem::size_of::<MagmaArmDeviceInfo>() as u32,
                p_next: 0,
            },
            shader_present,
            tiler_present,
            l2_present,
            gpu_id,
            gpu_rev,
            csf_id,
            l2_features,
            tiler_features,
            mem_features,
            mmu_features,
            thread_features,
            max_threads,
            thread_max_workgroup_size,
            thread_max_barrier_size,
            coherency_features,
            selected_coherency,
            as_present,
            csg_slot_count,
            cs_slot_count,
            cs_reg_count,
            scoreboard_slot_count,
        }
    }
}

impl MagmaAmdgpuDeviceInfo {
    pub fn new(
        max_engine_clock: u64,
        max_memory_clock: u64,
        ids_flags: u64,
        chip_rev: u32,
        external_rev: u32,
        family: u32,
        num_shader_engines: u32,
        num_shader_arrays_per_engine: u32,
        cu_active_number: u32,
        enabled_rb_pipes_mask: u32,
        num_rb_pipes: u32,
        vram_type: u32,
        vram_bit_width: u32,
        wave_front_size: u32,
        num_shader_visible_vgprs: u32,
        num_cu_per_sh: u32,
        num_tcc_blocks: u32,
    ) -> MagmaAmdgpuDeviceInfo {
        MagmaAmdgpuDeviceInfo {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::AmdgpuDeviceInfo as u32,
                size: std::mem::size_of::<MagmaAmdgpuDeviceInfo>() as u32,
                p_next: 0,
            },
            max_engine_clock,
            max_memory_clock,
            ids_flags,
            chip_rev,
            external_rev,
            family,
            num_shader_engines,
            num_shader_arrays_per_engine,
            cu_active_number,
            enabled_rb_pipes_mask,
            num_rb_pipes,
            vram_type,
            vram_bit_width,
            wave_front_size,
            num_shader_visible_vgprs,
            num_cu_per_sh,
            num_tcc_blocks,
        }
    }
}

impl MagmaPhysicalDeviceMemoryType {
    pub fn new(property_flags: u32, heap_idx: u32) -> MagmaPhysicalDeviceMemoryType {
        MagmaPhysicalDeviceMemoryType {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::PhysicalDeviceMemoryType as u32,
                size: std::mem::size_of::<MagmaPhysicalDeviceMemoryType>() as u32,
                p_next: 0,
            },
            property_flags,
            heap_idx,
        }
    }
}

impl MagmaPhysicalDeviceMemoryHeap {
    pub fn new(heap_size: u64, heap_flags: u64) -> MagmaPhysicalDeviceMemoryHeap {
        MagmaPhysicalDeviceMemoryHeap {
            header: MagmaStructureTypeHeader {
                stype: MagmaStructureType::PhysicalDeviceMemoryHeap as u32,
                size: std::mem::size_of::<MagmaPhysicalDeviceMemoryHeap>() as u32,
                p_next: 0,
            },
            heap_size,
            heap_flags,
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
