// Copyright 2026 The Magma GPU Project
// SPDX-License-Identifier: MIT

use std::cmp::min;
use std::mem::size_of;
use std::os::fd::BorrowedFd;

use zerocopy::FromZeros;
use zerocopy::KnownLayout;

use crate::error::Error;
use crate::error::Result;
use crate::ioctl_readwrite;
use crate::ioctl_write_ptr;
use crate::sys::linux::bindings::drm_bindings::DRM_COMMAND_BASE;
use crate::sys::linux::bindings::drm_bindings::DRM_IOCTL_BASE;
use crate::sys::linux::bindings::xe_bindings::*;

/// Derived from:
///   - `src/intel/dev/intel_device_info.c`
///   - `src/drivers/gpu/drm/xe/xe_pat.c`
///   - `intel/vulkan/anv_device.c`
const INTEL_GFX_VER_GFX12: u16 = 12;
const INTEL_GFX_VER_XELPG_MINOR: u16 = 70;
const INTEL_GFX_VER_XE2: u16 = 20;
const GFX12_PAT_WB_COHERENT: u16 = 0;
const GFX12_PAT_WC: u16 = 1;
const XELPG_PAT_WC: u16 = 1;
const XELPG_PAT_WB_1WAY_COHERENT: u16 = 3;

const XE2_PAT_WB_INCOHERENT: u16 = 0;
const XE2_PAT_WB_1WAY_COHERENT: u16 = 1;
const XE2_PAT_SCANOUT_DISPLAY_TRANSIENT: u16 = 6;

/// Determines the platform-specific Page Attribute Table (PAT) index for `drm_xe_vm_bind_op`.
pub fn determine_pat_index(
    ip_ver_major: u16,
    ip_ver_minor: u16,
    is_cached: bool,
    is_scanout: bool,
) -> u16 {
    if ip_ver_major >= INTEL_GFX_VER_XE2 {
        if is_scanout {
            XE2_PAT_SCANOUT_DISPLAY_TRANSIENT
        } else if is_cached {
            XE2_PAT_WB_1WAY_COHERENT
        } else {
            XE2_PAT_WB_INCOHERENT
        }
    } else if ip_ver_major == INTEL_GFX_VER_GFX12 && ip_ver_minor >= INTEL_GFX_VER_XELPG_MINOR {
        if is_cached && !is_scanout {
            XELPG_PAT_WB_1WAY_COHERENT
        } else {
            XELPG_PAT_WC
        }
    } else if is_cached && !is_scanout {
        GFX12_PAT_WB_COHERENT
    } else {
        GFX12_PAT_WC
    }
}

#[derive(Default)]
pub struct XeMemoryInfo {
    pub sysmem_size: u64,
    pub vram_size: u64,
    pub sysmem_used: u64,
    pub vram_used: u64,
    pub vram_cpu_visible_size: u64,
    pub vram_cpu_visible_used: u64,
    pub sysmem_instance: u16,
    pub vram_instance: u16,
}

ioctl_readwrite!(
    drm_ioctl_xe_device_query,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_DEVICE_QUERY,
    drm_xe_device_query
);

ioctl_readwrite!(
    drm_ioctl_xe_gem_create,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_GEM_CREATE,
    drm_xe_gem_create
);

ioctl_readwrite!(
    drm_ioctl_xe_gem_mmap_offset,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_GEM_MMAP_OFFSET,
    drm_xe_gem_mmap_offset
);

ioctl_readwrite!(
    drm_ioctl_xe_vm_create,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_VM_CREATE,
    drm_xe_vm_create
);

ioctl_write_ptr!(
    drm_ioctl_xe_vm_destroy,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_VM_DESTROY,
    drm_xe_vm_destroy
);

ioctl_readwrite!(
    drm_ioctl_xe_vm_bind,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_VM_BIND,
    drm_xe_vm_bind
);

ioctl_readwrite!(
    drm_ioctl_xe_exec_queue_create,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_EXEC_QUEUE_CREATE,
    drm_xe_exec_queue_create
);

ioctl_write_ptr!(
    drm_ioctl_xe_exec_queue_destroy,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_EXEC_QUEUE_DESTROY,
    drm_xe_exec_queue_destroy
);

ioctl_write_ptr!(
    drm_ioctl_xe_exec,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_EXEC,
    drm_xe_exec
);

ioctl_readwrite!(
    drm_ioctl_xe_exec_queue_get_property,
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + DRM_XE_EXEC_QUEUE_GET_PROPERTY,
    drm_xe_exec_queue_get_property
);

pub fn xe_device_query<T, S, H>(fd: BorrowedFd<'_>, query_id: u32) -> Result<Box<T>>
where
    T: ?Sized + FromZeros + KnownLayout<PointerMetadata = usize>,
{
    let mut device_query = drm_xe_device_query {
        query: query_id,
        ..Default::default()
    };

    // SAFETY: `fd` is a valid DRM descriptor and `device_query` is stack-allocated.
    unsafe {
        drm_ioctl_xe_device_query(fd, &mut device_query)?;
    }

    let total_size = device_query.size as usize;
    if total_size == 0 {
        return T::new_box_zeroed_with_elems(0).map_err(|_| Error::MemoryError);
    }
    let header_size = size_of::<H>();
    let element_size = size_of::<S>();
    let count = total_size
        .saturating_sub(header_size)
        .div_ceil(element_size);

    let mut query_data = T::new_box_zeroed_with_elems(count).map_err(|_| Error::MemoryError)?;

    device_query.size = total_size as u32;
    device_query.data = &mut *query_data as *mut T as *mut () as u64;

    // SAFETY: `query_data` is sized to hold `total_size` bytes returned by the initial query.
    unsafe {
        drm_ioctl_xe_device_query(fd, &mut device_query)?;
    }

    Ok(query_data)
}

pub fn xe_query_config(fd: BorrowedFd<'_>) -> Result<(u64, u64, u32)> {
    let query_config = xe_device_query::<
        drm_xe_query_config<[__u64]>,
        __u64,
        drm_xe_query_config<[__u64; 0]>,
    >(fd, DRM_XE_DEVICE_QUERY_CONFIG)?;

    let num_params = min(query_config.num_params as usize, query_config.info.len());
    let config = &query_config.info[..num_params];

    let gtt_size = 1u64 << config[DRM_XE_QUERY_CONFIG_VA_BITS as usize];
    let mem_alignment = config[DRM_XE_QUERY_CONFIG_MIN_ALIGNMENT as usize];
    let flags = config
        .get(crate::sys::linux::bindings::xe_bindings::DRM_XE_QUERY_CONFIG_FLAGS as usize)
        .copied()
        .unwrap_or(0) as u32;
    Ok((gtt_size, mem_alignment, flags))
}

pub fn xe_query_gt_list(fd: BorrowedFd<'_>) -> Result<(u32, u16, u16, u16)> {
    let gt_list = xe_device_query::<
        drm_xe_query_gt_list<[drm_xe_gt]>,
        drm_xe_gt,
        drm_xe_query_gt_list<[drm_xe_gt; 0]>,
    >(fd, DRM_XE_DEVICE_QUERY_GT_LIST)?;

    let num_gt = min(gt_list.num_gt as usize, gt_list.gt_list.len());
    for gt in &gt_list.gt_list[..num_gt] {
        if gt.type_ as u32 == DRM_XE_QUERY_GT_TYPE_MAIN {
            return Ok((
                gt.reference_clock,
                gt.ip_ver_major,
                gt.ip_ver_minor,
                gt.ip_ver_rev,
            ));
        }
    }
    if num_gt > 0 {
        let gt = &gt_list.gt_list[0];
        return Ok((
            gt.reference_clock,
            gt.ip_ver_major,
            gt.ip_ver_minor,
            gt.ip_ver_rev,
        ));
    }
    Ok((0, 0, 0, 0))
}

pub fn xe_query_gt_topology(fd: BorrowedFd<'_>) -> Result<(u64, u32, u64, u32)> {
    let mut geo_dss_mask = 0u64;
    let mut geo_dss_num_bytes = 0u32;
    let mut eu_per_dss_mask = 0u64;
    let mut l3_banks = 0u32;

    let Ok(topo_bytes) = xe_device_query::<[u8], u8, ()>(fd, DRM_XE_DEVICE_QUERY_GT_TOPOLOGY)
    else {
        return Ok((geo_dss_mask, geo_dss_num_bytes, eu_per_dss_mask, l3_banks));
    };

    let mut offset = 0usize;
    while offset + 8 <= topo_bytes.len() {
        let gt_id = u16::from_ne_bytes([topo_bytes[offset], topo_bytes[offset + 1]]);
        let topo_type = u16::from_ne_bytes([topo_bytes[offset + 2], topo_bytes[offset + 3]]) as u32;
        let num_bytes = u32::from_ne_bytes([
            topo_bytes[offset + 4],
            topo_bytes[offset + 5],
            topo_bytes[offset + 6],
            topo_bytes[offset + 7],
        ]) as usize;

        offset += 8;
        if offset + num_bytes > topo_bytes.len() {
            break;
        }
        let mask = &topo_bytes[offset..offset + num_bytes];
        offset += num_bytes;

        if gt_id == 0 {
            match topo_type {
                DRM_XE_TOPO_DSS_GEOMETRY => {
                    let copy_len = min(num_bytes, 8);
                    geo_dss_mask = 0;
                    for (i, &b) in mask[..copy_len].iter().enumerate() {
                        geo_dss_mask |= (b as u64) << (8 * i);
                    }
                    geo_dss_num_bytes = copy_len as u32;
                }
                DRM_XE_TOPO_L3_BANK => {
                    for &b in mask {
                        l3_banks += b.count_ones();
                    }
                }
                DRM_XE_TOPO_EU_PER_DSS | DRM_XE_TOPO_SIMD16_EU_PER_DSS => {
                    let copy_len = min(num_bytes, 8);
                    for (i, &b) in mask[..copy_len].iter().enumerate() {
                        eu_per_dss_mask |= (b as u64) << (8 * i);
                    }
                }
                _ => {}
            }
        }
    }

    Ok((geo_dss_mask, geo_dss_num_bytes, eu_per_dss_mask, l3_banks))
}

pub fn xe_query_hwconfig(fd: BorrowedFd<'_>) -> Result<Vec<u32>> {
    match xe_device_query::<[u32], u32, ()>(fd, DRM_XE_DEVICE_QUERY_HWCONFIG) {
        Ok(hwconfig) => Ok(hwconfig.into_vec()),
        Err(_) => Ok(Vec::new()),
    }
}

pub fn xe_query_memory_regions(fd: BorrowedFd<'_>) -> Result<XeMemoryInfo> {
    let mut memory_info: XeMemoryInfo = Default::default();

    let query_mem_regions = xe_device_query::<
        drm_xe_query_mem_regions<[drm_xe_mem_region]>,
        drm_xe_mem_region,
        drm_xe_query_mem_regions<[drm_xe_mem_region; 0]>,
    >(fd, DRM_XE_DEVICE_QUERY_MEM_REGIONS)?;

    let num_regions = min(
        query_mem_regions.num_mem_regions as usize,
        query_mem_regions.mem_regions.len(),
    );
    let mem_regions = &query_mem_regions.mem_regions[..num_regions];
    for region in mem_regions {
        match region.mem_class as u32 {
            DRM_XE_MEM_REGION_CLASS_SYSMEM => {
                if memory_info.sysmem_size != 0 {
                    return Err(Error::InternalError);
                }
                memory_info.sysmem_size = region.total_size;
                memory_info.sysmem_used = region.used;
                memory_info.sysmem_instance = region.instance;
            }
            DRM_XE_MEM_REGION_CLASS_VRAM => {
                if memory_info.vram_size != 0 || memory_info.vram_cpu_visible_size != 0 {
                    return Err(Error::InternalError);
                }
                memory_info.vram_cpu_visible_size = region.cpu_visible_size;
                memory_info.vram_size = region.total_size - region.cpu_visible_size;
                memory_info.vram_cpu_visible_used = region.cpu_visible_used;
                memory_info.vram_used = region.used - region.cpu_visible_used;
                memory_info.vram_instance = region.instance;
            }
            _ => return Err(Error::Unimplemented),
        }
    }

    Ok(memory_info)
}

pub fn xe_gem_create(
    fd: BorrowedFd<'_>,
    size: u64,
    flags: u32,
    cpu_caching: u16,
    placement: u32,
    is_protected: bool,
) -> Result<u32> {
    let mut gem_create = drm_xe_gem_create {
        size,
        flags,
        cpu_caching,
        placement,
        ..Default::default()
    };
    let mut pxp_ext: drm_xe_ext_set_property = Default::default();
    if is_protected {
        pxp_ext.base.name = DRM_XE_GEM_CREATE_EXTENSION_SET_PROPERTY;
        pxp_ext.property = DRM_XE_GEM_CREATE_SET_PROPERTY_PXP_TYPE;
        pxp_ext.__bindgen_anon_1.value = DRM_XE_PXP_TYPE_HWDRM as u64;
        gem_create.extensions = &pxp_ext as *const drm_xe_ext_set_property as u64;
    }
    // SAFETY: `fd` is valid and `gem_create` / `pxp_ext` outlive the ioctl call.
    unsafe {
        drm_ioctl_xe_gem_create(fd, &mut gem_create)?;
    }
    Ok(gem_create.handle)
}

pub fn xe_gem_mmap_offset(fd: BorrowedFd<'_>, gem_handle: u32) -> Result<u64> {
    let mut xe_offset = drm_xe_gem_mmap_offset {
        handle: gem_handle,
        ..Default::default()
    };
    // SAFETY: `fd` is valid and `xe_offset` is stack-allocated.
    unsafe {
        drm_ioctl_xe_gem_mmap_offset(fd, &mut xe_offset)?;
    }
    Ok(xe_offset.offset)
}

pub fn xe_vm_create(fd: BorrowedFd<'_>, flags: u32) -> Result<u32> {
    let mut vm_create = drm_xe_vm_create {
        flags,
        ..Default::default()
    };
    // SAFETY: `fd` is valid and `vm_create` is stack-allocated.
    unsafe {
        drm_ioctl_xe_vm_create(fd, &mut vm_create)?;
    }
    Ok(vm_create.vm_id)
}

pub fn xe_vm_destroy(fd: BorrowedFd<'_>, vm_id: u32) -> Result<()> {
    let destroy = drm_xe_vm_destroy {
        vm_id,
        ..Default::default()
    };
    // SAFETY: `fd` is valid and `destroy` is stack-allocated.
    unsafe {
        drm_ioctl_xe_vm_destroy(fd, &destroy)?;
    }
    Ok(())
}

pub fn xe_vm_bind(
    fd: BorrowedFd<'_>,
    vm_id: u32,
    exec_queue_id: u32,
    binds: &[drm_xe_vm_bind_op],
    syncs: &[drm_xe_sync],
) -> Result<()> {
    let mut vm_bind = drm_xe_vm_bind {
        vm_id,
        exec_queue_id,
        num_binds: binds.len() as u32,
        num_syncs: syncs.len() as u32,
        syncs: if syncs.is_empty() {
            0
        } else {
            syncs.as_ptr() as u64
        },
        ..Default::default()
    };
    if binds.len() == 1 {
        vm_bind.__bindgen_anon_1.bind = binds[0];
    } else if binds.len() > 1 {
        vm_bind.__bindgen_anon_1.vector_of_binds = binds.as_ptr() as u64;
    }

    // SAFETY: `fd` is valid and `binds` / `syncs` slices remain valid for the duration of the ioctl.
    unsafe {
        drm_ioctl_xe_vm_bind(fd, &mut vm_bind)?;
    }
    Ok(())
}

pub fn xe_exec_queue_create(fd: BorrowedFd<'_>, vm_id: u32, engine_class: u16) -> Result<u32> {
    let instance = drm_xe_engine_class_instance {
        engine_class,
        engine_instance: 0,
        gt_id: 0,
        pad: 0,
    };
    let mut create = drm_xe_exec_queue_create {
        instances: &instance as *const _ as u64,
        width: 1,
        num_placements: 1,
        vm_id,
        ..Default::default()
    };
    // SAFETY: `fd` is valid and `instance` outlives the ioctl call.
    unsafe {
        drm_ioctl_xe_exec_queue_create(fd, &mut create)?;
    }
    Ok(create.exec_queue_id)
}

pub fn xe_exec_queue_destroy(fd: BorrowedFd<'_>, exec_queue_id: u32) -> Result<()> {
    let destroy = drm_xe_exec_queue_destroy {
        exec_queue_id,
        ..Default::default()
    };
    // SAFETY: `fd` is valid and `destroy` is stack-allocated.
    unsafe {
        drm_ioctl_xe_exec_queue_destroy(fd, &destroy)?;
    }
    Ok(())
}

pub fn xe_exec(
    fd: BorrowedFd<'_>,
    exec_queue_id: u32,
    num_batch_buffer: u16,
    address: u64,
    syncs: &[drm_xe_sync],
) -> Result<()> {
    let exec = drm_xe_exec {
        exec_queue_id,
        num_batch_buffer,
        address,
        num_syncs: syncs.len() as u32,
        syncs: syncs.as_ptr() as u64,
        ..Default::default()
    };
    // SAFETY: `fd` is valid and `syncs` slice remains valid for the duration of the ioctl.
    unsafe {
        drm_ioctl_xe_exec(fd, &exec)?;
    }
    Ok(())
}

pub fn xe_exec_queue_get_property(
    fd: BorrowedFd<'_>,
    exec_queue_id: u32,
    property: u32,
) -> Result<u64> {
    let mut prop = drm_xe_exec_queue_get_property {
        extensions: 0,
        exec_queue_id,
        property,
        value: 0,
        reserved: [0; 2],
    };
    // SAFETY: `fd` is valid and `prop` is stack-allocated.
    unsafe {
        drm_ioctl_xe_exec_queue_get_property(fd, &mut prop)?;
    }
    Ok(prop.value)
}
