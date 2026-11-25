// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

mod amd;
mod macros;
mod wddm;

pub use amd::Amd;
pub use wddm::enumerate_devices;
pub use wddm::VendorPrivateData;
pub use wddm::WindowsDevice as PlatformDevice;
pub use wddm::WindowsPhysicalDevice as PlatformPhysicalDevice;

use crate::defines::MagmaPollItem;
use crate::error::Error;
use crate::error::Result;

pub fn poll(_items: &mut [MagmaPollItem], _abs_timeout_ns: u64) -> Result<()> {
    Err(Error::Unimplemented)
}
