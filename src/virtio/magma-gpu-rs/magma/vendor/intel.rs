// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

//! Intel vendor-specific Magma query IDs and payloads matching `magma_intel.h`.

use zerocopy::FromBytes;
use zerocopy::Immutable;
use zerocopy::IntoBytes;
use zerocopy::KnownLayout;

pub const MAGMA_INTEL_QUERY_DEVICE_INFO: u64 = 10000;
pub const MAGMA_INTEL_QUERY_HWCONFIG: u64 = 10001;

pub const MAGMA_INTEL_CONFIG_FLAG_HAS_VRAM: u32 = 0x00000001;
pub const MAGMA_INTEL_CONFIG_FLAG_HAS_LOW_LATENCY: u32 = 0x00000002;
pub const MAGMA_INTEL_CONFIG_FLAG_HAS_CPU_ADDR_MIRROR: u32 = 0x00000004;
pub const MAGMA_INTEL_CONFIG_FLAG_HAS_NO_COMPRESSION_HINT: u32 = 0x00000008;
pub const MAGMA_INTEL_CONFIG_FLAG_HAS_STATE_CACHE_PERF_FIX: u32 = 0x00000010;
pub const MAGMA_INTEL_CONFIG_FLAG_HAS_PURGING_SUPPORT: u32 = 0x00000020;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, FromBytes, Immutable, IntoBytes, KnownLayout)]
pub struct MagmaIntelDeviceInfo {
    pub l3_banks: u32,
    pub geo_dss_num_bytes: u32,
    pub geo_dss_mask: u64,
    pub eu_per_dss_mask: u64,
    pub ip_ver_major: u32,
    pub ip_ver_minor: u32,
    pub ip_ver_rev: u32,
    pub config_flags: u32,
}
