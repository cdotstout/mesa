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
