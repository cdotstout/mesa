// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

mod bindings;
mod common;
mod drm;
mod macros;
mod virtgpu;
mod xe;

pub use common::enumerate_devices;
pub use common::PlatformDevice;
pub use common::PlatformPhysicalDevice;
pub use drm::*;
pub use virtgpu::virtgpu_enumerate_devices;
pub use xe::XePhysicalDevice;
