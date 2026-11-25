// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT
//
// Generated via:
//   https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/magma/gorgonzola
//
// Submit patches, do not hand-edit.
use crate::protocol::*;
pub struct Encoder<W> {
    writer: W,
    bytes_written: usize,
}

impl<W: std::io::Write> Encoder<W> {
    pub fn new(writer: W) -> Encoder<W> {
        Encoder {
            writer,
            bytes_written: 0,
        }
    }

    pub fn bytes_written(&self) -> usize {
        self.bytes_written
    }

    pub fn into_inner(self) -> W {
        self.writer
    }

    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    pub fn writer(&self) -> &W {
        &self.writer
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(bytes)?;
        self.bytes_written += bytes.len();
        Ok(())
    }
    pub fn encode_extensible_chain(&mut self, mut curr: u64) -> std::io::Result<()> {
        while curr != 0 {
            let header = unsafe { &*(curr as *const MagmaStructureTypeHeader) };
            let next = header.p_next as usize as u64;
            match header.stype {
                MAGMA_STRUCTURE_TYPE_CREATE_BUFFER_INFO => {
                    let mut ext = unsafe { *(curr as *const MagmaCreateBufferInfo) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                MAGMA_STRUCTURE_TYPE_BUFFER_PLACEMENT_INFO => {
                    let mut ext = unsafe { *(curr as *const MagmaBufferPlacementInfo) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                MAGMA_STRUCTURE_TYPE_DEVICE_CREATE_INFO => {
                    let mut ext = unsafe { *(curr as *const MagmaDeviceCreateInfo) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                MAGMA_STRUCTURE_TYPE_SUBMIT_SYNC_INFO => {
                    let ext = unsafe { &*(curr as *const MagmaWireSubmitSyncInfo) };
                    let mut hdr = MagmaWireSubmitSyncInfoHeader {
                        header: ext.header,
                        num_wait_semaphores: ext.num_wait_semaphores,
                        num_signal_semaphores: ext.num_signal_semaphores,
                    };
                    hdr.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&hdr))?;
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(
                        ext.wait_semaphores.as_slice(),
                    ))?;
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(
                        ext.signal_semaphores.as_slice(),
                    ))?;
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(ext.wait_points.as_slice()))?;
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(ext.signal_points.as_slice()))?;
                }
                MAGMA_STRUCTURE_TYPE_SUBMIT_INFO => {
                    let ext = unsafe { &*(curr as *const MagmaWireSubmitInfo) };
                    let mut hdr = MagmaWireSubmitInfoHeader {
                        header: ext.header,
                        flags: ext.flags,
                        _pad0: ext._pad0,
                        sync_info: MagmaWireSubmitSyncInfoHeader {
                            header: ext.sync_info.header,
                            num_wait_semaphores: ext.sync_info.num_wait_semaphores,
                            num_signal_semaphores: ext.sync_info.num_signal_semaphores,
                        },
                    };
                    hdr.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&hdr))?;
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(
                        ext.sync_info.wait_semaphores.as_slice(),
                    ))?;
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(
                        ext.sync_info.signal_semaphores.as_slice(),
                    ))?;
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(
                        ext.sync_info.wait_points.as_slice(),
                    ))?;
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(
                        ext.sync_info.signal_points.as_slice(),
                    ))?;
                }
                MAGMA_STRUCTURE_TYPE_SUBMIT_ADDRESS_SPACE_INFO => {
                    let mut ext = unsafe { *(curr as *const MagmaSubmitAddressSpaceInfo) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                MAGMA_STRUCTURE_TYPE_SUBMIT_BUFFER_INFO => {
                    let mut ext = unsafe { *(curr as *const MagmaWireSubmitBufferInfo) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                MAGMA_STRUCTURE_TYPE_SUBMIT_MAP_BUFFER_INFO => {
                    let mut ext = unsafe { *(curr as *const MagmaWireSubmitMapBufferInfo) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                MAGMA_STRUCTURE_TYPE_SUBMIT_UNMAP_BUFFER_INFO => {
                    let mut ext = unsafe { *(curr as *const MagmaSubmitUnmapBufferInfo) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                MAGMA_STRUCTURE_TYPE_SUBMIT_RESOURCES_INFO => {
                    let ext = unsafe { &*(curr as *const MagmaWireSubmitResourcesInfo) };
                    let mut hdr = MagmaWireSubmitResourcesInfoHeader {
                        header: ext.header,
                        num_resources: ext.num_resources,
                        _padding: ext._padding,
                    };
                    hdr.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&hdr))?;
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(ext.buffers.as_slice()))?;
                }
                MAGMA_STRUCTURE_TYPE_CREATE_SEMAPHORE_INFO => {
                    let mut ext = unsafe { *(curr as *const MagmaCreateSemaphoreInfo) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                MAGMA_STRUCTURE_TYPE_SYNC_PROPERTIES => {
                    let mut ext = unsafe { *(curr as *const MagmaSyncProperties) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                MAGMA_STRUCTURE_TYPE_VIRT_CAPABILITIES => {
                    let mut ext = unsafe { *(curr as *const MagmaVirtCapabilities) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                MAGMA_STRUCTURE_TYPE_PHYSICAL_DEVICE_INFO => {
                    let mut ext = unsafe { *(curr as *const MagmaPhysicalDeviceInfo) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                MAGMA_STRUCTURE_TYPE_ADDRESS_SPACE_PROPERTIES => {
                    let mut ext = unsafe { *(curr as *const MagmaAddressSpaceProperties) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                MAGMA_STRUCTURE_TYPE_TIMESTAMP_PROPERTIES => {
                    let mut ext = unsafe { *(curr as *const MagmaTimestampProperties) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                MAGMA_STRUCTURE_TYPE_MEMORY_HEAPS_PROPERTIES => {
                    let mut ext = unsafe { *(curr as *const MagmaMemoryHeapsProperties) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                MAGMA_STRUCTURE_TYPE_VIRTIO_SEQNO_INFO => {
                    let mut ext = unsafe { *(curr as *const MagmaVirtioSeqnoInfo) };
                    ext.header.p_next = if next != 0 { 1 } else { 0 };
                    self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;
                }
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "unknown stype",
                    ))
                }
            }
            curr = next;
        }
        Ok(())
    }
    pub fn encode_create_buffer_info(
        &mut self,
        msg: &MagmaCreateBufferInfo,
    ) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_buffer_placement_info(
        &mut self,
        msg: &MagmaBufferPlacementInfo,
    ) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_device_create_info(
        &mut self,
        msg: &MagmaDeviceCreateInfo,
    ) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_wire_submit_sync_info(
        &mut self,
        msg: &MagmaWireSubmitSyncInfo,
    ) -> std::io::Result<()> {
        let mut obj = MagmaWireSubmitSyncInfoHeader {
            header: msg.header,
            num_wait_semaphores: msg.num_wait_semaphores,
            num_signal_semaphores: msg.num_signal_semaphores,
        };
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg.wait_semaphores.as_slice()))?;
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg.signal_semaphores.as_slice()))?;
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg.wait_points.as_slice()))?;
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg.signal_points.as_slice()))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_wire_submit_info(&mut self, msg: &MagmaWireSubmitInfo) -> std::io::Result<()> {
        let mut obj = MagmaWireSubmitInfoHeader {
            header: msg.header,
            flags: msg.flags,
            _pad0: msg._pad0,
            sync_info: MagmaWireSubmitSyncInfoHeader {
                header: msg.sync_info.header,
                num_wait_semaphores: msg.sync_info.num_wait_semaphores,
                num_signal_semaphores: msg.sync_info.num_signal_semaphores,
            },
        };
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg.sync_info.wait_semaphores.as_slice()))?;
        self.write_bytes(zerocopy::IntoBytes::as_bytes(
            msg.sync_info.signal_semaphores.as_slice(),
        ))?;
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg.sync_info.wait_points.as_slice()))?;
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg.sync_info.signal_points.as_slice()))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_submit_address_space_info(
        &mut self,
        msg: &MagmaSubmitAddressSpaceInfo,
    ) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_wire_submit_buffer_info(
        &mut self,
        msg: &MagmaWireSubmitBufferInfo,
    ) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_wire_submit_map_buffer_info(
        &mut self,
        msg: &MagmaWireSubmitMapBufferInfo,
    ) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_submit_unmap_buffer_info(
        &mut self,
        msg: &MagmaSubmitUnmapBufferInfo,
    ) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_wire_submit_resources_info(
        &mut self,
        msg: &MagmaWireSubmitResourcesInfo,
    ) -> std::io::Result<()> {
        let mut obj = MagmaWireSubmitResourcesInfoHeader {
            header: msg.header,
            num_resources: msg.num_resources,
            _padding: msg._padding,
        };
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg.buffers.as_slice()))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_create_semaphore_info(
        &mut self,
        msg: &MagmaCreateSemaphoreInfo,
    ) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_sync_properties(&mut self, msg: &MagmaSyncProperties) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_virt_capabilities(&mut self, msg: &MagmaVirtCapabilities) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_physical_device_info(
        &mut self,
        msg: &MagmaPhysicalDeviceInfo,
    ) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_address_space_properties(
        &mut self,
        msg: &MagmaAddressSpaceProperties,
    ) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_timestamp_properties(
        &mut self,
        msg: &MagmaTimestampProperties,
    ) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_memory_heaps_properties(
        &mut self,
        msg: &MagmaMemoryHeapsProperties,
    ) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_virtio_seqno_info(&mut self, msg: &MagmaVirtioSeqnoInfo) -> std::io::Result<()> {
        let mut obj = *msg;
        let p_next = obj.header.p_next as usize as u64;
        obj.header.p_next = if p_next != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;
        if p_next != 0 {
            self.encode_extensible_chain(p_next)?;
        }
        Ok(())
    }
    pub fn encode_physical_device_create_device(
        &mut self,
        msg: &PhysicalDeviceCreateDevice,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_physical_device_create_device(
        &mut self,
        msg: &PhysicalDeviceCreateDeviceResp,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_device_create_buffer(&mut self, msg: &DeviceCreateBuffer) -> std::io::Result<()> {
        let mut cmd = *msg;
        let p_next_info = cmd.info.header.p_next as usize as u64;
        cmd.info.header.p_next = if p_next_info != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&cmd))?;
        if p_next_info != 0 {
            self.encode_extensible_chain(p_next_info)?;
        }
        Ok(())
    }
    pub fn encode_resp_device_create_buffer(
        &mut self,
        msg: &DeviceCreateBufferResp,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_device_create_address_space(
        &mut self,
        msg: &DeviceCreateAddressSpace,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_device_create_address_space(
        &mut self,
        msg: &DeviceCreateAddressSpaceResp,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_device_create_queue(&mut self, msg: &DeviceCreateQueue) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_device_create_queue(
        &mut self,
        msg: &DeviceCreateQueueResp,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_device_release(&mut self, msg: &DeviceRelease) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_physical_device_release(
        &mut self,
        msg: &PhysicalDeviceRelease,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_buffer_release(&mut self, msg: &BufferRelease) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_queue_release(&mut self, msg: &QueueRelease) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_address_space_release(
        &mut self,
        msg: &AddressSpaceRelease,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_virtio_create_ring(&mut self, msg: &VirtioCreateRing) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_queue_submit(&mut self, msg: &QueueSubmit) -> std::io::Result<()> {
        let mut cmd = QueueSubmitHeader {
            header: msg.header,
            queue: msg.queue,
            _pad0: msg._pad0,
            submit_info: MagmaWireSubmitInfoHeader {
                header: msg.submit_info.header,
                flags: msg.submit_info.flags,
                _pad0: msg.submit_info._pad0,
                sync_info: MagmaWireSubmitSyncInfoHeader {
                    header: msg.submit_info.sync_info.header,
                    num_wait_semaphores: msg.submit_info.sync_info.num_wait_semaphores,
                    num_signal_semaphores: msg.submit_info.sync_info.num_signal_semaphores,
                },
            },
        };
        let p_next_submit_info = cmd.submit_info.header.p_next as usize as u64;
        cmd.submit_info.header.p_next = if p_next_submit_info != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&cmd))?;
        self.write_bytes(zerocopy::IntoBytes::as_bytes(
            msg.submit_info.sync_info.wait_semaphores.as_slice(),
        ))?;
        self.write_bytes(zerocopy::IntoBytes::as_bytes(
            msg.submit_info.sync_info.signal_semaphores.as_slice(),
        ))?;
        self.write_bytes(zerocopy::IntoBytes::as_bytes(
            msg.submit_info.sync_info.wait_points.as_slice(),
        ))?;
        self.write_bytes(zerocopy::IntoBytes::as_bytes(
            msg.submit_info.sync_info.signal_points.as_slice(),
        ))?;
        if p_next_submit_info != 0 {
            self.encode_extensible_chain(p_next_submit_info)?;
        }
        Ok(())
    }
    pub fn encode_resp_queue_submit(&mut self, msg: &QueueSubmitResp) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_device_create_semaphore(
        &mut self,
        msg: &DeviceCreateSemaphore,
    ) -> std::io::Result<()> {
        let mut cmd = *msg;
        let p_next_info = cmd.info.header.p_next as usize as u64;
        cmd.info.header.p_next = if p_next_info != 0 { 1 } else { 0 };
        self.write_bytes(zerocopy::IntoBytes::as_bytes(&cmd))?;
        if p_next_info != 0 {
            self.encode_extensible_chain(p_next_info)?;
        }
        Ok(())
    }
    pub fn encode_resp_device_create_semaphore(
        &mut self,
        msg: &DeviceCreateSemaphoreResp,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_semaphore_release(&mut self, msg: &SemaphoreRelease) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_semaphore_signal(&mut self, msg: &SemaphoreSignal) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_semaphore_signal(
        &mut self,
        msg: &SemaphoreSignalResp,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_semaphore_timeline_wait(
        &mut self,
        msg: &SemaphoreTimelineWait,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_semaphore_timeline_wait(
        &mut self,
        msg: &SemaphoreTimelineWaitResp,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_semaphore_timeline_signal(
        &mut self,
        msg: &SemaphoreTimelineSignal,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_semaphore_timeline_signal(
        &mut self,
        msg: &SemaphoreTimelineSignalResp,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_semaphore_timeline_query(
        &mut self,
        msg: &SemaphoreTimelineQuery,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_semaphore_timeline_query(
        &mut self,
        msg: &SemaphoreTimelineQueryResp,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_queue_check_status(&mut self, msg: &QueueCheckStatus) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_resp_queue_check_status(
        &mut self,
        msg: &QueueCheckStatusResp,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_virtio_ping(&mut self, msg: &VirtioPing) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_virtio_create_fence(&mut self, msg: &VirtioCreateFence) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_virtio_semaphore_release(
        &mut self,
        msg: &VirtioSemaphoreRelease,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_virtio_buffer_release(
        &mut self,
        msg: &VirtioBufferRelease,
    ) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_virtio_import_buffer(&mut self, msg: &VirtioImportBuffer) -> std::io::Result<()> {
        self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))
    }
    pub fn encode_msg(&mut self, msg: &MagmaProtocol) -> std::io::Result<()> {
        match msg {
            MagmaProtocol::PhysicalDeviceCreateDevice(msg) => {
                self.encode_physical_device_create_device(msg)
            }
            MagmaProtocol::DeviceCreateBuffer(msg) => self.encode_device_create_buffer(msg),
            MagmaProtocol::DeviceCreateAddressSpace(msg) => {
                self.encode_device_create_address_space(msg)
            }
            MagmaProtocol::DeviceCreateQueue(msg) => self.encode_device_create_queue(msg),
            MagmaProtocol::DeviceRelease(msg) => self.encode_device_release(msg),
            MagmaProtocol::PhysicalDeviceRelease(msg) => self.encode_physical_device_release(msg),
            MagmaProtocol::BufferRelease(msg) => self.encode_buffer_release(msg),
            MagmaProtocol::QueueRelease(msg) => self.encode_queue_release(msg),
            MagmaProtocol::AddressSpaceRelease(msg) => self.encode_address_space_release(msg),
            MagmaProtocol::VirtioCreateRing(msg) => self.encode_virtio_create_ring(msg),
            MagmaProtocol::QueueSubmit(msg) => self.encode_queue_submit(msg),
            MagmaProtocol::DeviceCreateSemaphore(msg) => self.encode_device_create_semaphore(msg),
            MagmaProtocol::SemaphoreRelease(msg) => self.encode_semaphore_release(msg),
            MagmaProtocol::SemaphoreSignal(msg) => self.encode_semaphore_signal(msg),
            MagmaProtocol::SemaphoreTimelineWait(msg) => self.encode_semaphore_timeline_wait(msg),
            MagmaProtocol::SemaphoreTimelineSignal(msg) => {
                self.encode_semaphore_timeline_signal(msg)
            }
            MagmaProtocol::SemaphoreTimelineQuery(msg) => self.encode_semaphore_timeline_query(msg),
            MagmaProtocol::QueueCheckStatus(msg) => self.encode_queue_check_status(msg),
            MagmaProtocol::VirtioPing(msg) => self.encode_virtio_ping(msg),
            MagmaProtocol::VirtioCreateFence(msg) => self.encode_virtio_create_fence(msg),
            MagmaProtocol::VirtioSemaphoreRelease(msg) => self.encode_virtio_semaphore_release(msg),
            MagmaProtocol::VirtioBufferRelease(msg) => self.encode_virtio_buffer_release(msg),
            MagmaProtocol::VirtioImportBuffer(msg) => self.encode_virtio_import_buffer(msg),
            MagmaProtocol::PhysicalDeviceCreateDeviceResp(msg) => {
                self.encode_resp_physical_device_create_device(msg)
            }
            MagmaProtocol::DeviceCreateBufferResp(msg) => {
                self.encode_resp_device_create_buffer(msg)
            }
            MagmaProtocol::DeviceCreateAddressSpaceResp(msg) => {
                self.encode_resp_device_create_address_space(msg)
            }
            MagmaProtocol::DeviceCreateQueueResp(msg) => self.encode_resp_device_create_queue(msg),
            MagmaProtocol::QueueSubmitResp(msg) => self.encode_resp_queue_submit(msg),
            MagmaProtocol::DeviceCreateSemaphoreResp(msg) => {
                self.encode_resp_device_create_semaphore(msg)
            }
            MagmaProtocol::SemaphoreSignalResp(msg) => self.encode_resp_semaphore_signal(msg),
            MagmaProtocol::SemaphoreTimelineWaitResp(msg) => {
                self.encode_resp_semaphore_timeline_wait(msg)
            }
            MagmaProtocol::SemaphoreTimelineSignalResp(msg) => {
                self.encode_resp_semaphore_timeline_signal(msg)
            }
            MagmaProtocol::SemaphoreTimelineQueryResp(msg) => {
                self.encode_resp_semaphore_timeline_query(msg)
            }
            MagmaProtocol::QueueCheckStatusResp(msg) => self.encode_resp_queue_check_status(msg),
        }
    }

    #[allow(dead_code)]
    pub fn encode_extensible_struct(&mut self, msg: &MagmaExtensibleStruct) -> std::io::Result<()> {
        match msg {
            MagmaExtensibleStruct::CreateBufferInfo(msg) => self.encode_create_buffer_info(msg),
            MagmaExtensibleStruct::BufferPlacementInfo(msg) => {
                self.encode_buffer_placement_info(msg)
            }
            MagmaExtensibleStruct::DeviceCreateInfo(msg) => self.encode_device_create_info(msg),
            MagmaExtensibleStruct::WireSubmitSyncInfo(msg) => {
                self.encode_wire_submit_sync_info(msg)
            }
            MagmaExtensibleStruct::WireSubmitInfo(msg) => self.encode_wire_submit_info(msg),
            MagmaExtensibleStruct::SubmitAddressSpaceInfo(msg) => {
                self.encode_submit_address_space_info(msg)
            }
            MagmaExtensibleStruct::WireSubmitBufferInfo(msg) => {
                self.encode_wire_submit_buffer_info(msg)
            }
            MagmaExtensibleStruct::WireSubmitMapBufferInfo(msg) => {
                self.encode_wire_submit_map_buffer_info(msg)
            }
            MagmaExtensibleStruct::SubmitUnmapBufferInfo(msg) => {
                self.encode_submit_unmap_buffer_info(msg)
            }
            MagmaExtensibleStruct::WireSubmitResourcesInfo(msg) => {
                self.encode_wire_submit_resources_info(msg)
            }
            MagmaExtensibleStruct::CreateSemaphoreInfo(msg) => {
                self.encode_create_semaphore_info(msg)
            }
            MagmaExtensibleStruct::SyncProperties(msg) => self.encode_sync_properties(msg),
            MagmaExtensibleStruct::VirtCapabilities(msg) => self.encode_virt_capabilities(msg),
            MagmaExtensibleStruct::PhysicalDeviceInfo(msg) => self.encode_physical_device_info(msg),
            MagmaExtensibleStruct::AddressSpaceProperties(msg) => {
                self.encode_address_space_properties(msg)
            }
            MagmaExtensibleStruct::TimestampProperties(msg) => {
                self.encode_timestamp_properties(msg)
            }
            MagmaExtensibleStruct::MemoryHeapsProperties(msg) => {
                self.encode_memory_heaps_properties(msg)
            }
            MagmaExtensibleStruct::VirtioSeqnoInfo(msg) => self.encode_virtio_seqno_info(msg),
        }
    }
}

impl<'a> Encoder<&'a mut [u8]> {
    pub fn from_slice(buf: &'a mut [u8]) -> Encoder<&'a mut [u8]> {
        Encoder::new(buf)
    }
}
