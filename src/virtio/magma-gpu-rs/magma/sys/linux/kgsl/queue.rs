// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use crate::defines::MagmaSubmitInfo;
use crate::defines::MagmaSubmitPayload;
use crate::error::Error;
use crate::error::Result;
use crate::sys::linux::kgsl::ioctl;
use crate::traits::BackendAddressSpace;
use crate::traits::BackendPhysicalDevice;
use crate::traits::BackendQueue;
use crate::traits::GenericQueue;

pub struct KgslQueue {
    physical_device: Arc<dyn BackendPhysicalDevice>,
    address_space: Arc<dyn BackendAddressSpace>,
    context_id: u32,
}

impl KgslQueue {
    pub fn new(
        physical_device: Arc<dyn BackendPhysicalDevice>,
        address_space: Arc<dyn BackendAddressSpace>,
    ) -> Result<KgslQueue> {
        let context_id = ioctl::drawctxt_create(physical_device.as_fd().unwrap())?;
        Ok(KgslQueue {
            physical_device,
            address_space,
            context_id,
        })
    }
}

impl Drop for KgslQueue {
    fn drop(&mut self) {
        ioctl::drawctxt_destroy(self.physical_device.as_fd().unwrap(), self.context_id);
    }
}

impl GenericQueue for KgslQueue {
    fn submit_command(&self, submit_info: &MagmaSubmitInfo) -> Result<()> {
        let mut has_exec = false;
        let mut had_map_op = false;
        for payload in submit_info.payloads() {
            match payload {
                MagmaSubmitPayload::AddressSpace(_) | MagmaSubmitPayload::Buffer(_) => {
                    has_exec = true;
                }
                MagmaSubmitPayload::MapBuffer(map_info) => {
                    had_map_op = true;
                    let buffer = map_info.buffer().ok_or(Error::InvalidArgs)?;
                    self.address_space.map_buffer_gpu(
                        buffer.inner(),
                        map_info.buffer_offset,
                        map_info.gpu_va,
                        map_info.size,
                        map_info.flags,
                    )?;
                }
                MagmaSubmitPayload::UnmapBuffer(unmap_info) => {
                    had_map_op = true;
                    self.address_space
                        .unmap_buffer_gpu(unmap_info.gpu_va, unmap_info.size)?;
                }
                _ => return Err(Error::Unimplemented),
            }
        }
        if !has_exec && had_map_op {
            return Ok(());
        }
        ioctl::gpu_command_submit(
            self.physical_device.as_fd().unwrap(),
            self.context_id,
            submit_info,
        )
    }
}

impl BackendQueue for KgslQueue {}
