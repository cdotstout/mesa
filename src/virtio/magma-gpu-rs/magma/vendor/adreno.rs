// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

//! Qualcomm Adreno vendor-specific Magma query IDs and payloads matching `magma_adreno.h`.

use zerocopy::FromBytes;
use zerocopy::Immutable;
use zerocopy::IntoBytes;
use zerocopy::KnownLayout;

pub const MAGMA_ADRENO_QUERY_DEVICE_INFO: u64 = 10000;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, FromBytes, Immutable, IntoBytes, KnownLayout)]
pub struct MagmaAdrenoDeviceInfo {
    pub chip_id: u64,
    pub gmem_size: u64,
    pub gmem_base: u64,
    pub gpu_id: u32,
    pub highest_bank_bit: u32,
    pub ubwc_swizzle: u32,
    pub macrotile_mode: u32,
    pub raytracing: u32,
    pub flags: u32,
}
