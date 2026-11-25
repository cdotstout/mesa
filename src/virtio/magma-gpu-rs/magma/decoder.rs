// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT
//
// Generated via:
//   https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/magma/gorgonzola
//
// Submit patches, do not hand-edit.
use crate::protocol::*;
use magma_gpu::util::Reader;

#[allow(dead_code)]
fn decode_wire_submit_sync_info(reader: &mut Reader) -> Option<MagmaWireSubmitSyncInfo> {
    let hdr: MagmaWireSubmitSyncInfoHeader = reader.read_try_obj().ok()?;
    let wait_semaphores = reader.read_vec(hdr.num_wait_semaphores as usize).ok()?;
    let signal_semaphores = reader.read_vec(hdr.num_signal_semaphores as usize).ok()?;
    let wait_points = reader.read_vec(hdr.num_wait_semaphores as usize).ok()?;
    let signal_points = reader.read_vec(hdr.num_signal_semaphores as usize).ok()?;
    Some(MagmaWireSubmitSyncInfo {
        header: hdr.header,
        num_wait_semaphores: hdr.num_wait_semaphores,
        num_signal_semaphores: hdr.num_signal_semaphores,
        wait_semaphores,
        signal_semaphores,
        wait_points,
        signal_points,
    })
}

#[allow(dead_code)]
fn decode_wire_submit_info(reader: &mut Reader) -> Option<MagmaWireSubmitInfo> {
    let hdr: MagmaWireSubmitInfoHeader = reader.read_try_obj().ok()?;
    let sync_info_wait_semaphores = reader
        .read_vec(hdr.sync_info.num_wait_semaphores as usize)
        .ok()?;
    let sync_info_signal_semaphores = reader
        .read_vec(hdr.sync_info.num_signal_semaphores as usize)
        .ok()?;
    let sync_info_wait_points = reader.read_vec(hdr.sync_info.num_wait_semaphores as usize).ok()?;
    let sync_info_signal_points = reader
        .read_vec(hdr.sync_info.num_signal_semaphores as usize)
        .ok()?;
    Some(MagmaWireSubmitInfo {
        header: hdr.header,
        flags: hdr.flags,
        _pad0: hdr._pad0,
        sync_info: MagmaWireSubmitSyncInfo {
            header: hdr.sync_info.header,
            num_wait_semaphores: hdr.sync_info.num_wait_semaphores,
            num_signal_semaphores: hdr.sync_info.num_signal_semaphores,
            wait_semaphores: sync_info_wait_semaphores,
            signal_semaphores: sync_info_signal_semaphores,
            wait_points: sync_info_wait_points,
            signal_points: sync_info_signal_points,
        },
    })
}

#[allow(dead_code)]
fn decode_wire_submit_resources_info(reader: &mut Reader) -> Option<MagmaWireSubmitResourcesInfo> {
    let hdr: MagmaWireSubmitResourcesInfoHeader = reader.read_try_obj().ok()?;
    let buffers = reader.read_vec(hdr.num_resources as usize).ok()?;
    Some(MagmaWireSubmitResourcesInfo {
        header: hdr.header,
        num_resources: hdr.num_resources,
        _padding: hdr._padding,
        buffers,
    })
}

#[allow(dead_code)]
fn decode_queue_submit(reader: &mut Reader) -> Option<QueueSubmit> {
    let hdr: QueueSubmitHeader = reader.read_try_obj().ok()?;
    let submit_info_sync_info_wait_semaphores = reader
        .read_vec(hdr.submit_info.sync_info.num_wait_semaphores as usize)
        .ok()?;
    let submit_info_sync_info_signal_semaphores = reader
        .read_vec(hdr.submit_info.sync_info.num_signal_semaphores as usize)
        .ok()?;
    let submit_info_sync_info_wait_points = reader
        .read_vec(hdr.submit_info.sync_info.num_wait_semaphores as usize)
        .ok()?;
    let submit_info_sync_info_signal_points = reader
        .read_vec(hdr.submit_info.sync_info.num_signal_semaphores as usize)
        .ok()?;
    Some(QueueSubmit {
        header: hdr.header,
        queue: hdr.queue,
        _pad0: hdr._pad0,
        submit_info: MagmaWireSubmitInfo {
            header: hdr.submit_info.header,
            flags: hdr.submit_info.flags,
            _pad0: hdr.submit_info._pad0,
            sync_info: MagmaWireSubmitSyncInfo {
                header: hdr.submit_info.sync_info.header,
                num_wait_semaphores: hdr.submit_info.sync_info.num_wait_semaphores,
                num_signal_semaphores: hdr.submit_info.sync_info.num_signal_semaphores,
                wait_semaphores: submit_info_sync_info_wait_semaphores,
                signal_semaphores: submit_info_sync_info_signal_semaphores,
                wait_points: submit_info_sync_info_wait_points,
                signal_points: submit_info_sync_info_signal_points,
            },
        },
    })
}

pub fn decode(reader: &mut Reader) -> Option<MagmaProtocol> {
    if reader.available_bytes() < std::mem::size_of::<MagmaCommandHeader>() {
        return None;
    }
    let header = reader.peek_obj::<MagmaCommandHeader>().ok()?;
    match header.opcode {
        MAGMA_OPCODE_PHYSICAL_DEVICE_CREATE_DEVICE => Some(
            MagmaProtocol::PhysicalDeviceCreateDevice(reader.read_try_obj().ok()?),
        ),
        MAGMA_OPCODE_DEVICE_CREATE_BUFFER => Some(MagmaProtocol::DeviceCreateBuffer(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_DEVICE_CREATE_ADDRESS_SPACE => Some(MagmaProtocol::DeviceCreateAddressSpace(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_DEVICE_CREATE_QUEUE => Some(MagmaProtocol::DeviceCreateQueue(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_DEVICE_RELEASE => {
            Some(MagmaProtocol::DeviceRelease(reader.read_try_obj().ok()?))
        }
        MAGMA_OPCODE_PHYSICAL_DEVICE_RELEASE => Some(MagmaProtocol::PhysicalDeviceRelease(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_BUFFER_RELEASE => {
            Some(MagmaProtocol::BufferRelease(reader.read_try_obj().ok()?))
        }
        MAGMA_OPCODE_QUEUE_RELEASE => {
            Some(MagmaProtocol::QueueRelease(reader.read_try_obj().ok()?))
        }
        MAGMA_OPCODE_ADDRESS_SPACE_RELEASE => Some(MagmaProtocol::AddressSpaceRelease(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_VIRTIO_CREATE_RING => {
            Some(MagmaProtocol::VirtioCreateRing(reader.read_try_obj().ok()?))
        }
        MAGMA_OPCODE_QUEUE_SUBMIT => Some(MagmaProtocol::QueueSubmit(decode_queue_submit(reader)?)),
        MAGMA_OPCODE_DEVICE_CREATE_SEMAPHORE => Some(MagmaProtocol::DeviceCreateSemaphore(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_SEMAPHORE_RELEASE => {
            Some(MagmaProtocol::SemaphoreRelease(reader.read_try_obj().ok()?))
        }
        MAGMA_OPCODE_SEMAPHORE_SIGNAL => {
            Some(MagmaProtocol::SemaphoreSignal(reader.read_try_obj().ok()?))
        }
        MAGMA_OPCODE_SEMAPHORE_TIMELINE_WAIT => Some(MagmaProtocol::SemaphoreTimelineWait(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_SEMAPHORE_TIMELINE_SIGNAL => Some(MagmaProtocol::SemaphoreTimelineSignal(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_SEMAPHORE_TIMELINE_QUERY => Some(MagmaProtocol::SemaphoreTimelineQuery(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_QUEUE_CHECK_STATUS => {
            Some(MagmaProtocol::QueueCheckStatus(reader.read_try_obj().ok()?))
        }
        MAGMA_OPCODE_VIRTIO_PING => Some(MagmaProtocol::VirtioPing(reader.read_try_obj().ok()?)),
        MAGMA_OPCODE_VIRTIO_CREATE_FENCE => Some(MagmaProtocol::VirtioCreateFence(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_VIRTIO_SEMAPHORE_RELEASE => Some(MagmaProtocol::VirtioSemaphoreRelease(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_VIRTIO_BUFFER_RELEASE => Some(MagmaProtocol::VirtioBufferRelease(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_VIRTIO_IMPORT_BUFFER => Some(MagmaProtocol::VirtioImportBuffer(
            reader.read_try_obj().ok()?,
        )),

        MAGMA_OPCODE_RESP_PHYSICAL_DEVICE_CREATE_DEVICE => Some(
            MagmaProtocol::PhysicalDeviceCreateDeviceResp(reader.read_try_obj().ok()?),
        ),
        MAGMA_OPCODE_RESP_DEVICE_CREATE_BUFFER => Some(MagmaProtocol::DeviceCreateBufferResp(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_RESP_DEVICE_CREATE_ADDRESS_SPACE => Some(
            MagmaProtocol::DeviceCreateAddressSpaceResp(reader.read_try_obj().ok()?),
        ),
        MAGMA_OPCODE_RESP_DEVICE_CREATE_QUEUE => Some(MagmaProtocol::DeviceCreateQueueResp(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_RESP_QUEUE_SUBMIT => {
            Some(MagmaProtocol::QueueSubmitResp(reader.read_try_obj().ok()?))
        }
        MAGMA_OPCODE_RESP_DEVICE_CREATE_SEMAPHORE => Some(
            MagmaProtocol::DeviceCreateSemaphoreResp(reader.read_try_obj().ok()?),
        ),
        MAGMA_OPCODE_RESP_SEMAPHORE_SIGNAL => Some(MagmaProtocol::SemaphoreSignalResp(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_OPCODE_RESP_SEMAPHORE_TIMELINE_WAIT => Some(
            MagmaProtocol::SemaphoreTimelineWaitResp(reader.read_try_obj().ok()?),
        ),
        MAGMA_OPCODE_RESP_SEMAPHORE_TIMELINE_SIGNAL => Some(
            MagmaProtocol::SemaphoreTimelineSignalResp(reader.read_try_obj().ok()?),
        ),
        MAGMA_OPCODE_RESP_SEMAPHORE_TIMELINE_QUERY => Some(
            MagmaProtocol::SemaphoreTimelineQueryResp(reader.read_try_obj().ok()?),
        ),
        MAGMA_OPCODE_RESP_QUEUE_CHECK_STATUS => Some(MagmaProtocol::QueueCheckStatusResp(
            reader.read_try_obj().ok()?,
        )),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn decode_extensible_struct(reader: &mut Reader) -> Option<MagmaExtensibleStruct> {
    if reader.available_bytes() < std::mem::size_of::<MagmaStructureTypeHeader>() {
        return None;
    }
    let header = reader.peek_obj::<MagmaStructureTypeHeader>().ok()?;
    match header.stype {
        MAGMA_STRUCTURE_TYPE_CREATE_BUFFER_INFO => Some(MagmaExtensibleStruct::CreateBufferInfo(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_STRUCTURE_TYPE_BUFFER_PLACEMENT_INFO => Some(
            MagmaExtensibleStruct::BufferPlacementInfo(reader.read_try_obj().ok()?),
        ),
        MAGMA_STRUCTURE_TYPE_DEVICE_CREATE_INFO => Some(MagmaExtensibleStruct::DeviceCreateInfo(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO => Some(MagmaExtensibleStruct::WireSubmitSyncInfo(
            decode_wire_submit_sync_info(reader)?,
        )),
        MAGMA_STRUCTURE_TYPE_SUBMIT_INFO => Some(MagmaExtensibleStruct::WireSubmitInfo(
            decode_wire_submit_info(reader)?,
        )),
        MAGMA_STRUCTURE_TYPE_SUBMIT_ADDRESS_SPACE_INFO => Some(
            MagmaExtensibleStruct::SubmitAddressSpaceInfo(reader.read_try_obj().ok()?),
        ),
        MAGMA_STRUCTURE_TYPE_SUBMIT_BUFFER_INFO => Some(
            MagmaExtensibleStruct::WireSubmitBufferInfo(reader.read_try_obj().ok()?),
        ),
        MAGMA_STRUCTURE_TYPE_SUBMIT_MAP_BUFFER_INFO => Some(
            MagmaExtensibleStruct::WireSubmitMapBufferInfo(reader.read_try_obj().ok()?),
        ),
        MAGMA_STRUCTURE_TYPE_SUBMIT_UNMAP_BUFFER_INFO => Some(
            MagmaExtensibleStruct::SubmitUnmapBufferInfo(reader.read_try_obj().ok()?),
        ),
        MAGMA_STRUCTURE_TYPE_SUBMIT_RESOURCES_INFO => Some(
            MagmaExtensibleStruct::WireSubmitResourcesInfo(decode_wire_submit_resources_info(reader)?),
        ),
        MAGMA_STRUCTURE_TYPE_CREATE_SEMAPHORE_INFO => Some(
            MagmaExtensibleStruct::CreateSemaphoreInfo(reader.read_try_obj().ok()?),
        ),
        MAGMA_STRUCTURE_TYPE_SYNC_PROPERTIES => Some(MagmaExtensibleStruct::SyncProperties(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_STRUCTURE_TYPE_VIRT_CAPABILITIES => Some(MagmaExtensibleStruct::VirtCapabilities(
            reader.read_try_obj().ok()?,
        )),
        MAGMA_STRUCTURE_TYPE_PHYSICAL_DEVICE_INFO => Some(
            MagmaExtensibleStruct::PhysicalDeviceInfo(reader.read_try_obj().ok()?),
        ),
        MAGMA_STRUCTURE_TYPE_ADDRESS_SPACE_PROPERTIES => Some(
            MagmaExtensibleStruct::AddressSpaceProperties(reader.read_try_obj().ok()?),
        ),
        MAGMA_STRUCTURE_TYPE_TIMESTAMP_PROPERTIES => Some(
            MagmaExtensibleStruct::TimestampProperties(reader.read_try_obj().ok()?),
        ),
        MAGMA_STRUCTURE_TYPE_MEMORY_HEAPS_PROPERTIES => Some(
            MagmaExtensibleStruct::MemoryHeapsProperties(reader.read_try_obj().ok()?),
        ),
        MAGMA_STRUCTURE_TYPE_VIRTIO_SEQNO_INFO => Some(MagmaExtensibleStruct::VirtioSeqnoInfo(
            reader.read_try_obj().ok()?,
        )),
        _ => None,
    }
}
