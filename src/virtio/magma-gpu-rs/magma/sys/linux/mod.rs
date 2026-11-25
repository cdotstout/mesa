// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

mod amdgpu;
mod bindings;
mod common;
mod drm;
mod kgsl;
mod macros;
mod panthor;
mod virtgpu;
mod xe;

pub use amdgpu::AmdGpuPhysicalDevice;
pub use common::enumerate_devices;
pub use common::LinuxDescriptor;
pub use common::PlatformDevice;
pub use common::PlatformPhysicalDevice;
pub use drm::*;
pub use kgsl::KgslPhysicalDevice;
pub use panthor::PanthorPhysicalDevice;
pub use virtgpu::virtgpu_enumerate_devices;
pub use xe::XePhysicalDevice;

use crate::defines::MagmaPollCondition;
use crate::defines::MagmaPollItem;
use crate::defines::MagmaPollType;
use crate::error::Error;
use crate::error::Result;
use crate::traits::BackendSemaphore;

pub fn poll(items: &mut [MagmaPollItem], abs_timeout_ns: u64) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }

    let mut sems: Vec<std::sync::Arc<dyn BackendSemaphore>> = Vec::with_capacity(items.len());
    for item in items.iter_mut() {
        item.result = MagmaPollCondition::empty();
        if item.poll_type != MagmaPollType::Semaphore {
            return Err(Error::Unimplemented);
        }
        let sem = item.semaphore.as_ref().ok_or(Error::InvalidArgs)?;
        sems.push(sem.semaphore().clone());
    }

    let sem_refs: Vec<&std::sync::Arc<dyn BackendSemaphore>> = sems.iter().collect();
    let idx = sems[0].wait_any(&sem_refs, abs_timeout_ns)?;
    if let Some(item) = items.get_mut(idx) {
        item.result |= MagmaPollCondition::Signaled;
    }
    Ok(())
}
