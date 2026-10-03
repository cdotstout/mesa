use crate::device::PhysicalDevice;
use crate::error::Error;
use crate::error::Result;

pub trait PlatformDevice {}

pub trait PlatformPhysicalDevice {}

pub fn enumerate_devices() -> Result<Vec<PhysicalDevice>> {
    Err(Error::Unimplemented)
}
