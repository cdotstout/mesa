// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT
//
// Generated via:
//   https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/magma/gorgonzola
//
// Submit patches, do not hand-edit.

#![allow(unused_imports)]
#![allow(dead_code)]

use bitflags::bitflags;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, TryFromBytes};

pub const MAGMA_OPCODE_PHYSICAL_DEVICE_CREATE_DEVICE: u32 = 1;
pub const MAGMA_OPCODE_DEVICE_CREATE_BUFFER: u32 = 2;
pub const MAGMA_OPCODE_DEVICE_CREATE_ADDRESS_SPACE: u32 = 3;
pub const MAGMA_OPCODE_DEVICE_CREATE_QUEUE: u32 = 4;
pub const MAGMA_OPCODE_DEVICE_RELEASE: u32 = 5;
pub const MAGMA_OPCODE_PHYSICAL_DEVICE_RELEASE: u32 = 6;
pub const MAGMA_OPCODE_BUFFER_RELEASE: u32 = 7;
pub const MAGMA_OPCODE_QUEUE_RELEASE: u32 = 8;
pub const MAGMA_OPCODE_ADDRESS_SPACE_RELEASE: u32 = 9;
pub const MAGMA_OPCODE_VIRTIO_CREATE_RING: u32 = 10;
pub const MAGMA_OPCODE_QUEUE_SUBMIT: u32 = 11;
pub const MAGMA_OPCODE_DEVICE_CREATE_SEMAPHORE: u32 = 12;
pub const MAGMA_OPCODE_SEMAPHORE_RELEASE: u32 = 13;
pub const MAGMA_OPCODE_SEMAPHORE_SIGNAL: u32 = 14;
pub const MAGMA_OPCODE_SEMAPHORE_TIMELINE_WAIT: u32 = 15;
pub const MAGMA_OPCODE_SEMAPHORE_TIMELINE_SIGNAL: u32 = 16;
pub const MAGMA_OPCODE_SEMAPHORE_TIMELINE_QUERY: u32 = 17;
pub const MAGMA_OPCODE_QUEUE_CHECK_STATUS: u32 = 18;
pub const MAGMA_OPCODE_VIRTIO_PING: u32 = 19;
pub const MAGMA_OPCODE_VIRTIO_CREATE_FENCE: u32 = 20;
pub const MAGMA_OPCODE_VIRTIO_SEMAPHORE_RELEASE: u32 = 21;
pub const MAGMA_OPCODE_VIRTIO_BUFFER_RELEASE: u32 = 22;
pub const MAGMA_OPCODE_VIRTIO_IMPORT_BUFFER: u32 = 23;

pub const MAGMA_OPCODE_RESP_PHYSICAL_DEVICE_CREATE_DEVICE: u32 = 0x8000_0001;
pub const MAGMA_OPCODE_RESP_DEVICE_CREATE_BUFFER: u32 = 0x8000_0002;
pub const MAGMA_OPCODE_RESP_DEVICE_CREATE_ADDRESS_SPACE: u32 = 0x8000_0003;
pub const MAGMA_OPCODE_RESP_DEVICE_CREATE_QUEUE: u32 = 0x8000_0004;
pub const MAGMA_OPCODE_RESP_QUEUE_SUBMIT: u32 = 0x8000_000b;
pub const MAGMA_OPCODE_RESP_DEVICE_CREATE_SEMAPHORE: u32 = 0x8000_000c;
pub const MAGMA_OPCODE_RESP_SEMAPHORE_SIGNAL: u32 = 0x8000_000e;
pub const MAGMA_OPCODE_RESP_SEMAPHORE_TIMELINE_WAIT: u32 = 0x8000_000f;
pub const MAGMA_OPCODE_RESP_SEMAPHORE_TIMELINE_SIGNAL: u32 = 0x8000_0010;
pub const MAGMA_OPCODE_RESP_SEMAPHORE_TIMELINE_QUERY: u32 = 0x8000_0011;
pub const MAGMA_OPCODE_RESP_QUEUE_CHECK_STATUS: u32 = 0x8000_0012;

pub const MAGMA_MAX_PHYSICAL_DEVICES: usize = 8;
pub const MAGMA_MAX_QUEUES: usize = 16;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TryFromBytes, IntoBytes, Immutable)]
#[repr(u16)]
pub enum MagmaVendorId {
    #[default]
    Undefined = 0,
    Amd = 4098,
    Arm = 5045,
    VirtGpu = 6900,
    Qualcomm = 21523,
    Intel = 32902,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TryFromBytes, IntoBytes, Immutable)]
#[repr(i32)]
pub enum MagmaStatus {
    #[default]
    Success = 0,
    InternalError = -1,
    InvalidArgs = -2,
    AccessDenied = -3,
    MemoryError = -4,
    ContextKilled = -5,
    TimedOut = -6,
    Unimplemented = -7,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TryFromBytes, IntoBytes, Immutable)]
#[repr(i32)]
pub enum VirtioGpuBlobPreference {
    #[default]
    Undefined = 0,
    Guest = 1,
    Host = 2,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TryFromBytes, IntoBytes, Immutable)]
#[repr(u32)]
pub enum MagmaSemaphoreType {
    #[default]
    Binary = 0,
    Timeline = 1,
}

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, IntoBytes, FromBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct MagmaQueueFlags(u32);
bitflags! {
    impl MagmaQueueFlags: u32 {
        const Graphics = 1;
        const Compute = 2;
        const Transfer = 4;
        const SparseBinding = 8;
        const Protected = 16;
    }
}

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, IntoBytes, FromBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct MagmaGpuMapFlags(u64);
bitflags! {
    impl MagmaGpuMapFlags: u64 {
        const Read = 1;
        const Write = 2;
        const Execute = 4;
        const GrowUp = 8;
        const GrowDown = 16;
    }
}

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, IntoBytes, FromBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct MagmaBufferFlag(u32);
bitflags! {
    impl MagmaBufferFlag: u32 {
        const External = 1;
        const Scanout = 2;
        const Cached = 4;
        const Protected = 8;
    }
}

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, IntoBytes, FromBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct MagmaSyncCapability(u32);
bitflags! {
    impl MagmaSyncCapability: u32 {
        const Binary = 1;
        const Timeline = 2;
        const CpuWait = 4;
        const CpuSignal = 8;
        const ExportSyncFile = 16;
        const ImportSyncFile = 32;
    }
}

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, IntoBytes, FromBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct MagmaSemaphoreUseFlags(u32);
bitflags! {
    impl MagmaSemaphoreUseFlags: u32 {
        const Exportable = 1;
        const Fence = 2;
    }
}

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, IntoBytes, FromBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct MagmaSemaphoreWaitFlags(u32);
bitflags! {
    impl MagmaSemaphoreWaitFlags: u32 {
        const WaitPending = 1;
    }
}

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, IntoBytes, FromBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct MagmaHandleType(u32);
bitflags! {
    impl MagmaHandleType: u32 {
        const MemOpaqueFd = 1;
        const MemDmabuf = 2;
        const MemOpaqueWin32 = 3;
        const MemShm = 4;
        const MemZircon = 5;
        const SignalOpaqueFd = 16;
        const SignalSyncFd = 32;
        const SignalOpaqueWin32 = 48;
        const SignalZircon = 64;
        const SignalEventFd = 80;
    }
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaPciBusInfo {
    pub domain: u16,
    pub subvendor_id: u16,
    pub subdevice_id: u16,
    pub revision_id: u8,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub _padding: [u8; 6],
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaCreateQueueInfo {
    pub queue_family_idx: u32,
    pub priority: u32,
    pub flags: u32,
    pub _padding: u32,
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaQueueFamilyProperties {
    pub queue_flags: MagmaQueueFlags,
    pub queue_count: u32,
}

pub const MAGMA_STRUCTURE_TYPE_CREATE_BUFFER_INFO: u32 = 1;
pub const MAGMA_STRUCTURE_TYPE_DEVICE_CREATE_INFO: u32 = 2;
pub const MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO: u32 = 3;
pub const MAGMA_STRUCTURE_TYPE_SUBMIT_INFO: u32 = 4;
pub const MAGMA_STRUCTURE_TYPE_SUBMIT_ADDRESS_SPACE_INFO: u32 = 5;
pub const MAGMA_STRUCTURE_TYPE_SUBMIT_BUFFER_INFO: u32 = 6;
pub const MAGMA_STRUCTURE_TYPE_SUBMIT_MAP_BUFFER_INFO: u32 = 9;
pub const MAGMA_STRUCTURE_TYPE_SUBMIT_UNMAP_BUFFER_INFO: u32 = 10;
pub const MAGMA_STRUCTURE_TYPE_SUBMIT_RESOURCES_INFO: u32 = 18;
pub const MAGMA_STRUCTURE_TYPE_CREATE_SEMAPHORE_INFO: u32 = 7;
pub const MAGMA_STRUCTURE_TYPE_SYNC_PROPERTIES: u32 = 8;
pub const MAGMA_STRUCTURE_TYPE_VIRT_CAPABILITIES: u32 = 65537;
pub const MAGMA_STRUCTURE_TYPE_PHYSICAL_DEVICE_INFO: u32 = 65538;
pub const MAGMA_STRUCTURE_TYPE_ADDRESS_SPACE_PROPERTIES: u32 = 12;
pub const MAGMA_STRUCTURE_TYPE_TIMESTAMP_PROPERTIES: u32 = 13;
pub const MAGMA_STRUCTURE_TYPE_VIRTIO_SEQNO_INFO: u32 = 65541;

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromBytes, IntoBytes, Immutable)]
#[repr(u32)]
pub enum MagmaStructureType {
    CreateBufferInfo = 1,
    DeviceCreateInfo = 2,
    SubmitSyncInfo = 3,
    SubmitInfo = 4,
    SubmitAddressSpaceInfo = 5,
    SubmitBufferInfo = 6,
    SubmitMapBufferInfo = 9,
    SubmitUnmapBufferInfo = 10,
    SubmitResourcesInfo = 18,
    CreateSemaphoreInfo = 7,
    SyncProperties = 8,
    VirtCapabilities = 65537,
    PhysicalDeviceInfo = 65538,
    AddressSpaceProperties = 12,
    TimestampProperties = 13,
    VirtioSeqnoInfo = 65541,
}

#[derive(Debug, Default, Clone, Copy, FromBytes, IntoBytes, Immutable)]
#[repr(C, align(8))]
pub struct MagmaStructureTypeHeader {
    pub stype: u32,
    pub size: u32,
    pub p_next: u64,
}

/// Marker trait indicating that structure `Self` is allowed to extend structure `Root`.
///
/// # Safety
/// Implementors must only be declared when the specification allows `Self` to extend `Root`.
pub unsafe trait MagmaExtends<Root> {}

/// Trait implemented by extensible structures indicating their structure type.
///
/// # Safety
/// Implementors must guarantee that `STRUCTURE_TYPE` matches the actual header's stype
/// and that the structure begins with a valid `MagmaStructureTypeHeader`.
pub unsafe trait MagmaTaggedStructure {
    const STRUCTURE_TYPE: MagmaStructureType;
    fn header(&self) -> &MagmaStructureTypeHeader;
    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader;

    /// Find an extension in the `p_next` chain matching type `T`.
    fn get_extension<T: MagmaTaggedStructure + MagmaExtends<Self>>(&self) -> Option<&T>
    where
        Self: Sized,
    {
        let mut curr = self.header().p_next as usize;
        while curr != 0 {
            let header = unsafe { &*(curr as *const MagmaStructureTypeHeader) };
            if header.stype == T::STRUCTURE_TYPE as u32 {
                return Some(unsafe { &*(curr as *const T) });
            }
            curr = header.p_next as usize;
        }
        None
    }

    /// Find a mutable extension in the `p_next` chain matching type `T`.
    ///
    /// # Safety
    /// Caller must ensure exclusive access to the structures in the chain.
    unsafe fn get_extension_mut<T: MagmaTaggedStructure + MagmaExtends<Self>>(
        &mut self,
    ) -> Option<&mut T>
    where
        Self: Sized,
    {
        let mut curr = self.header().p_next as usize;
        while curr != 0 {
            let header = &mut *(curr as *mut MagmaStructureTypeHeader);
            if header.stype == T::STRUCTURE_TYPE as u32 {
                return Some(&mut *(curr as *mut T));
            }
            curr = header.p_next as usize;
        }
        None
    }

    /// Append an extension structure to the end of the `p_next` chain.
    ///
    /// # Safety
    /// `next` must point to a valid extensible structure whose memory remains valid
    /// while the chain is referenced.
    unsafe fn push_next<T: MagmaTaggedStructure + MagmaExtends<Self>>(&mut self, next: &mut T)
    where
        Self: Sized,
    {
        let mut curr: *mut MagmaStructureTypeHeader = self.header_mut();
        while ((*curr).p_next as usize) != 0 {
            curr = ((*curr).p_next as usize) as *mut MagmaStructureTypeHeader;
        }
        (*curr).p_next = (next.header_mut() as *mut MagmaStructureTypeHeader) as usize as u64;
    }
}

#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaCreateBufferInfo {
    pub header: MagmaStructureTypeHeader,
    pub alignment: u32,
    pub common_flags: u32,
    pub vendor_flags: u32,
    pub _pad0: u32,
    pub size: u64,
}

unsafe impl MagmaTaggedStructure for MagmaCreateBufferInfo {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::CreateBufferInfo;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaDeviceCreateInfo {
    pub header: MagmaStructureTypeHeader,
    pub flags: u32,
    pub _padding: u32,
}

unsafe impl MagmaTaggedStructure for MagmaDeviceCreateInfo {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::DeviceCreateInfo;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaWireSubmitSyncInfoHeader {
    pub header: MagmaStructureTypeHeader,
    pub num_wait_semaphores: u32,
    pub num_signal_semaphores: u32,
}

#[derive(Debug, Default, Clone)]
#[repr(C)]
pub struct MagmaWireSubmitSyncInfo {
    pub header: MagmaStructureTypeHeader,
    pub num_wait_semaphores: u32,
    pub num_signal_semaphores: u32,
    pub wait_semaphores: Vec<u32>,
    pub signal_semaphores: Vec<u32>,
    pub wait_points: Vec<u64>,
    pub signal_points: Vec<u64>,
}

unsafe impl MagmaTaggedStructure for MagmaWireSubmitSyncInfo {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::SubmitSyncInfo;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaWireSubmitInfoHeader {
    pub header: MagmaStructureTypeHeader,
    pub flags: u32,
    pub _pad0: u32,
    pub sync_info: MagmaWireSubmitSyncInfoHeader,
}

#[derive(Debug, Default, Clone)]
#[repr(C)]
pub struct MagmaWireSubmitInfo {
    pub header: MagmaStructureTypeHeader,
    pub flags: u32,
    pub _pad0: u32,
    pub sync_info: MagmaWireSubmitSyncInfo,
}

unsafe impl MagmaTaggedStructure for MagmaWireSubmitInfo {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::SubmitInfo;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaSubmitAddressSpaceInfo {
    pub header: MagmaStructureTypeHeader,
    pub command_va: u64,
    pub length: u64,
}

unsafe impl MagmaTaggedStructure for MagmaSubmitAddressSpaceInfo {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::SubmitAddressSpaceInfo;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
unsafe impl MagmaExtends<MagmaWireSubmitInfo> for MagmaSubmitAddressSpaceInfo {}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaWireSubmitBufferInfo {
    pub header: MagmaStructureTypeHeader,
    pub command_buffer: u32,
    pub _pad0: u32,
    pub start_offset: u64,
    pub length: u64,
}

unsafe impl MagmaTaggedStructure for MagmaWireSubmitBufferInfo {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::SubmitBufferInfo;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
unsafe impl MagmaExtends<MagmaWireSubmitInfo> for MagmaWireSubmitBufferInfo {}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaWireSubmitMapBufferInfo {
    pub header: MagmaStructureTypeHeader,
    pub buffer: u32,
    pub _pad0: u32,
    pub buffer_offset: u64,
    pub gpu_va: u64,
    pub size: u64,
    pub flags: MagmaGpuMapFlags,
}

unsafe impl MagmaTaggedStructure for MagmaWireSubmitMapBufferInfo {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::SubmitMapBufferInfo;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
unsafe impl MagmaExtends<MagmaWireSubmitInfo> for MagmaWireSubmitMapBufferInfo {}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaSubmitUnmapBufferInfo {
    pub header: MagmaStructureTypeHeader,
    pub gpu_va: u64,
    pub size: u64,
}

unsafe impl MagmaTaggedStructure for MagmaSubmitUnmapBufferInfo {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::SubmitUnmapBufferInfo;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
unsafe impl MagmaExtends<MagmaWireSubmitInfo> for MagmaSubmitUnmapBufferInfo {}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaWireSubmitResourcesInfoHeader {
    pub header: MagmaStructureTypeHeader,
    pub num_resources: u32,
    pub _padding: u32,
}

#[derive(Debug, Default, Clone)]
#[repr(C)]
pub struct MagmaWireSubmitResourcesInfo {
    pub header: MagmaStructureTypeHeader,
    pub num_resources: u32,
    pub _padding: u32,
    pub buffers: Vec<u32>,
}

unsafe impl MagmaTaggedStructure for MagmaWireSubmitResourcesInfo {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::SubmitResourcesInfo;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
unsafe impl MagmaExtends<MagmaWireSubmitInfo> for MagmaWireSubmitResourcesInfo {}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaCreateSemaphoreInfo {
    pub header: MagmaStructureTypeHeader,
    pub semaphore_type: u32,
    pub use_flags: MagmaSemaphoreUseFlags,
    pub initial_point: u64,
}

unsafe impl MagmaTaggedStructure for MagmaCreateSemaphoreInfo {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::CreateSemaphoreInfo;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaSyncProperties {
    pub header: MagmaStructureTypeHeader,
    pub capabilities: u32,
    pub _pad0: u32,
    pub max_timeline_value_difference: u64,
}

unsafe impl MagmaTaggedStructure for MagmaSyncProperties {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::SyncProperties;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaVirtCapabilities {
    pub header: MagmaStructureTypeHeader,
    pub capset_version: u32,
    pub num_physical_devices: u32,
    pub blob_preference: VirtioGpuBlobPreference,
    pub _pad0: u32,
    pub ring_buffer_len: u64,
}

unsafe impl MagmaTaggedStructure for MagmaVirtCapabilities {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::VirtCapabilities;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaPhysicalDeviceInfo {
    pub header: MagmaStructureTypeHeader,
    pub bus_type: u32,
    pub vendor_id: MagmaVendorId,
    pub device_id: u16,
    pub flags: u32,
    pub pci_bus_info: MagmaPciBusInfo,
    pub queue_family_count: u32,
    pub queue_families: [MagmaQueueFamilyProperties; MAGMA_MAX_QUEUES],
}

unsafe impl MagmaTaggedStructure for MagmaPhysicalDeviceInfo {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::PhysicalDeviceInfo;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
unsafe impl MagmaExtends<MagmaVirtCapabilities> for MagmaPhysicalDeviceInfo {}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaAddressSpaceProperties {
    pub header: MagmaStructureTypeHeader,
    pub va_start: u64,
    pub va_size: u64,
    pub mem_alignment: u64,
}

unsafe impl MagmaTaggedStructure for MagmaAddressSpaceProperties {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::AddressSpaceProperties;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
unsafe impl MagmaExtends<MagmaPhysicalDeviceInfo> for MagmaAddressSpaceProperties {}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaTimestampProperties {
    pub header: MagmaStructureTypeHeader,
    pub timestamp_frequency: u64,
}

unsafe impl MagmaTaggedStructure for MagmaTimestampProperties {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::TimestampProperties;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
unsafe impl MagmaExtends<MagmaPhysicalDeviceInfo> for MagmaTimestampProperties {}
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct MagmaVirtioSeqnoInfo {
    pub header: MagmaStructureTypeHeader,
    pub ring_idx: u32,
    pub wait_seqno: u32,
}

unsafe impl MagmaTaggedStructure for MagmaVirtioSeqnoInfo {
    const STRUCTURE_TYPE: MagmaStructureType = MagmaStructureType::VirtioSeqnoInfo;

    fn header(&self) -> &MagmaStructureTypeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut MagmaStructureTypeHeader {
        &mut self.header
    }
}
unsafe impl MagmaExtends<MagmaWireSubmitInfo> for MagmaVirtioSeqnoInfo {}
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum MagmaExtensibleStruct {
    CreateBufferInfo(MagmaCreateBufferInfo),
    DeviceCreateInfo(MagmaDeviceCreateInfo),
    WireSubmitSyncInfo(MagmaWireSubmitSyncInfo),
    WireSubmitInfo(MagmaWireSubmitInfo),
    SubmitAddressSpaceInfo(MagmaSubmitAddressSpaceInfo),
    WireSubmitBufferInfo(MagmaWireSubmitBufferInfo),
    WireSubmitMapBufferInfo(MagmaWireSubmitMapBufferInfo),
    SubmitUnmapBufferInfo(MagmaSubmitUnmapBufferInfo),
    WireSubmitResourcesInfo(MagmaWireSubmitResourcesInfo),
    CreateSemaphoreInfo(MagmaCreateSemaphoreInfo),
    SyncProperties(MagmaSyncProperties),
    VirtCapabilities(MagmaVirtCapabilities),
    PhysicalDeviceInfo(MagmaPhysicalDeviceInfo),
    AddressSpaceProperties(MagmaAddressSpaceProperties),
    TimestampProperties(MagmaTimestampProperties),
    VirtioSeqnoInfo(MagmaVirtioSeqnoInfo),
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, FromBytes, IntoBytes, Immutable)]
pub struct MagmaCommandHeader {
    pub opcode: u32,
    pub size: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct PhysicalDeviceCreateDevice {
    pub header: MagmaCommandHeader,
    pub physical_device: u32,
    pub device: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct PhysicalDeviceCreateDeviceResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct DeviceCreateBuffer {
    pub header: MagmaCommandHeader,
    pub device: u32,
    pub _pad0: u32,
    pub info: MagmaCreateBufferInfo,
    pub buffer_out: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct DeviceCreateBufferResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct DeviceCreateAddressSpace {
    pub header: MagmaCommandHeader,
    pub device: u32,
    pub address_space: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct DeviceCreateAddressSpaceResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct DeviceCreateQueue {
    pub header: MagmaCommandHeader,
    pub device: u32,
    pub address_space: u32,
    pub info: MagmaCreateQueueInfo,
    pub queue: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct DeviceCreateQueueResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct DeviceRelease {
    pub header: MagmaCommandHeader,
    pub device: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct PhysicalDeviceRelease {
    pub header: MagmaCommandHeader,
    pub physical_device: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct BufferRelease {
    pub header: MagmaCommandHeader,
    pub buffer: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct QueueRelease {
    pub header: MagmaCommandHeader,
    pub queue: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct AddressSpaceRelease {
    pub header: MagmaCommandHeader,
    pub address_space: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct VirtioCreateRing {
    pub header: MagmaCommandHeader,
    pub ring_idx: u32,
    pub ring_blob_id: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct QueueSubmitHeader {
    pub header: MagmaCommandHeader,
    pub queue: u32,
    pub _pad0: u32,
    pub submit_info: MagmaWireSubmitInfoHeader,
}

#[derive(Debug, Default, Clone)]
pub struct QueueSubmit {
    pub header: MagmaCommandHeader,
    pub queue: u32,
    pub _pad0: u32,
    pub submit_info: MagmaWireSubmitInfo,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct QueueSubmitResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct DeviceCreateSemaphore {
    pub header: MagmaCommandHeader,
    pub device: u32,
    pub _pad0: u32,
    pub info: MagmaCreateSemaphoreInfo,
    pub semaphore: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct DeviceCreateSemaphoreResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SemaphoreRelease {
    pub header: MagmaCommandHeader,
    pub semaphore: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SemaphoreSignal {
    pub header: MagmaCommandHeader,
    pub semaphore: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SemaphoreSignalResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SemaphoreTimelineWait {
    pub header: MagmaCommandHeader,
    pub semaphore: u32,
    pub _pad0: u32,
    pub point: u64,
    pub timeout_ns: u64,
    pub flags: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SemaphoreTimelineWaitResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SemaphoreTimelineSignal {
    pub header: MagmaCommandHeader,
    pub semaphore: u32,
    pub _pad0: u32,
    pub point: u64,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SemaphoreTimelineSignalResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SemaphoreTimelineQuery {
    pub header: MagmaCommandHeader,
    pub semaphore: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct SemaphoreTimelineQueryResp {
    pub header: MagmaCommandHeader,
    pub point_out: u64,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct QueueCheckStatus {
    pub header: MagmaCommandHeader,
    pub queue: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct QueueCheckStatusResp {
    pub header: MagmaCommandHeader,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct VirtioPing {
    pub header: MagmaCommandHeader,
    pub ring_idx: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct VirtioCreateFence {
    pub header: MagmaCommandHeader,
    pub ring_idx: u32,
    pub guest_sync_id: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct VirtioSemaphoreRelease {
    pub header: MagmaCommandHeader,
    pub semaphore: u32,
    pub ring_idx: u32,
    pub wait_seqno: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct VirtioBufferRelease {
    pub header: MagmaCommandHeader,
    pub buffer: u32,
    pub ring_idx: u32,
    pub wait_seqno: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct VirtioImportBuffer {
    pub header: MagmaCommandHeader,
    pub res_handle: u32,
    pub buffer_out: u32,
    pub size: u64,
    pub common_flags: u32,
    pub _padding: u32,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum MagmaProtocol {
    PhysicalDeviceCreateDevice(PhysicalDeviceCreateDevice),
    DeviceCreateBuffer(DeviceCreateBuffer),
    DeviceCreateAddressSpace(DeviceCreateAddressSpace),
    DeviceCreateQueue(DeviceCreateQueue),
    DeviceRelease(DeviceRelease),
    PhysicalDeviceRelease(PhysicalDeviceRelease),
    BufferRelease(BufferRelease),
    QueueRelease(QueueRelease),
    AddressSpaceRelease(AddressSpaceRelease),
    VirtioCreateRing(VirtioCreateRing),
    QueueSubmit(QueueSubmit),
    DeviceCreateSemaphore(DeviceCreateSemaphore),
    SemaphoreRelease(SemaphoreRelease),
    SemaphoreSignal(SemaphoreSignal),
    SemaphoreTimelineWait(SemaphoreTimelineWait),
    SemaphoreTimelineSignal(SemaphoreTimelineSignal),
    SemaphoreTimelineQuery(SemaphoreTimelineQuery),
    QueueCheckStatus(QueueCheckStatus),
    VirtioPing(VirtioPing),
    VirtioCreateFence(VirtioCreateFence),
    VirtioSemaphoreRelease(VirtioSemaphoreRelease),
    VirtioBufferRelease(VirtioBufferRelease),
    VirtioImportBuffer(VirtioImportBuffer),

    PhysicalDeviceCreateDeviceResp(PhysicalDeviceCreateDeviceResp),
    DeviceCreateBufferResp(DeviceCreateBufferResp),
    DeviceCreateAddressSpaceResp(DeviceCreateAddressSpaceResp),
    DeviceCreateQueueResp(DeviceCreateQueueResp),
    QueueSubmitResp(QueueSubmitResp),
    DeviceCreateSemaphoreResp(DeviceCreateSemaphoreResp),
    SemaphoreSignalResp(SemaphoreSignalResp),
    SemaphoreTimelineWaitResp(SemaphoreTimelineWaitResp),
    SemaphoreTimelineSignalResp(SemaphoreTimelineSignalResp),
    SemaphoreTimelineQueryResp(SemaphoreTimelineQueryResp),
    QueueCheckStatusResp(QueueCheckStatusResp),
}
