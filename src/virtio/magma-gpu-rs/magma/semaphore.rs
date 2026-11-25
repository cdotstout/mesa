// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use magma_gpu::util::Handle as MagmaGpuHandle;

use crate::defines::MagmaSyncType;
use crate::error::Result;
use crate::protocol::MagmaSemaphoreUseFlags;
use crate::traits::BackendSemaphore;

#[derive(Clone)]
pub struct Semaphore {
    pub semaphore: Arc<dyn BackendSemaphore>,
}

impl Semaphore {
    pub fn new(semaphore: Arc<dyn BackendSemaphore>) -> Semaphore {
        Semaphore { semaphore }
    }

    pub fn wait(&self, abs_timeout_ns: u64) -> Result<()> {
        self.semaphore.wait(abs_timeout_ns)
    }

    pub fn signal(&self) -> Result<()> {
        self.semaphore.signal()
    }

    pub fn reset(&self) -> Result<()> {
        self.semaphore.reset()
    }

    pub fn timeline_wait(&self, point: u64, abs_timeout_ns: u64, flags: u32) -> Result<()> {
        self.semaphore.timeline_wait(point, abs_timeout_ns, flags)
    }

    pub fn timeline_signal(&self, point: u64) -> Result<()> {
        self.semaphore.timeline_signal(point)
    }

    pub fn timeline_query(&self) -> Result<u64> {
        self.semaphore.timeline_query()
    }

    pub fn export_fence(&self) -> Result<MagmaGpuHandle> {
        self.semaphore.export_fence()
    }

    pub fn import(&self, handle: MagmaGpuHandle) -> Result<()> {
        self.semaphore.import(handle)
    }

    pub fn as_raw_handle(&self) -> Option<u32> {
        self.semaphore.as_raw_handle()
    }

    pub fn use_flags(&self) -> MagmaSemaphoreUseFlags {
        self.semaphore.use_flags()
    }

    pub fn get_type(&self) -> MagmaSyncType {
        self.semaphore.get_type()
    }

    pub fn semaphore(&self) -> &Arc<dyn BackendSemaphore> {
        &self.semaphore
    }

    pub fn is_timeline(&self) -> bool {
        self.semaphore.is_timeline()
    }
}
