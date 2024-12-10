// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use crate::sys::windows::wddm::d3dkmt::D3dkmtHandle;
use crate::traits::AsVirtioSemaphore;
use crate::traits::BackendDevice;
use crate::traits::BackendSemaphore;
use crate::traits::GenericSemaphore;

pub struct WddmSemaphore {
    handle: D3dkmtHandle,
    _device: Arc<dyn BackendDevice>,
}

impl GenericSemaphore for WddmSemaphore {
    fn as_raw_handle(&self) -> Option<u32> {
        Some(self.handle)
    }
}

impl AsVirtioSemaphore for WddmSemaphore {}
impl BackendSemaphore for WddmSemaphore {}
