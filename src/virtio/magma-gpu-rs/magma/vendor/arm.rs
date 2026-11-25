// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

//! ARM Mali vendor-specific Magma query IDs and payloads matching `magma_arm.h`.

use zerocopy::FromBytes;
use zerocopy::Immutable;
use zerocopy::IntoBytes;
use zerocopy::KnownLayout;

pub const MAGMA_ARM_QUERY_DEVICE_INFO: u64 = 10000;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, FromBytes, Immutable, IntoBytes, KnownLayout)]
pub struct MagmaArmDeviceInfo {
    pub shader_present: u64,
    pub tiler_present: u64,
    pub l2_present: u64,
    pub gpu_id: u32,
    pub gpu_rev: u32,
    pub csf_id: u32,
    pub l2_features: u32,
    pub tiler_features: u32,
    pub mem_features: u32,
    pub mmu_features: u32,
    pub thread_features: u32,
    pub max_threads: u32,
    pub thread_max_workgroup_size: u32,
    pub thread_max_barrier_size: u32,
    pub coherency_features: u32,
    pub selected_coherency: u32,
    pub as_present: u32,
    pub csg_slot_count: u32,
    pub cs_slot_count: u32,
    pub cs_reg_count: u32,
    pub scoreboard_slot_count: u32,
}
