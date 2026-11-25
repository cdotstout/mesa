// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

//! AMDGPU vendor-specific Magma query IDs and payloads matching `magma_amdgpu.h`.

use zerocopy::FromBytes;
use zerocopy::Immutable;
use zerocopy::IntoBytes;
use zerocopy::KnownLayout;

pub const MAGMA_AMDGPU_QUERY_DEVICE_INFO: u64 = 10000;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, FromBytes, Immutable, IntoBytes, KnownLayout)]
pub struct MagmaAmdgpuDeviceInfo {
    pub max_engine_clock: u64,
    pub max_memory_clock: u64,
    pub ids_flags: u64,
    pub chip_rev: u32,
    pub external_rev: u32,
    pub family: u32,
    pub num_shader_engines: u32,
    pub num_shader_arrays_per_engine: u32,
    pub cu_active_number: u32,
    pub enabled_rb_pipes_mask: u32,
    pub num_rb_pipes: u32,
    pub vram_type: u32,
    pub vram_bit_width: u32,
    pub wave_front_size: u32,
    pub num_shader_visible_vgprs: u32,
    pub num_cu_per_sh: u32,
    pub num_tcc_blocks: u32,
}
