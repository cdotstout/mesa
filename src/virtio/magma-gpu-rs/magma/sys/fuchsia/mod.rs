use std::ffi::c_char;
use std::ffi::c_void;
use std::ffi::CStr;
use std::ffi::CString;
use std::ptr::null_mut;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;

use zerocopy::TryFromBytes;

use crate::defines::MagmaCreateBufferInfo;
use crate::defines::MagmaCreateQueueInfo;
use crate::defines::MagmaHeap;
use crate::defines::MagmaImportHandleInfo;
use crate::defines::MagmaMemoryType;
use crate::defines::MagmaPhysicalDeviceInfo;
use crate::defines::MagmaQueueFamilyProperties;
use crate::defines::MagmaQueueFlags;
use crate::defines::MAGMA_BUS_TYPE_PCI;
use crate::defines::MAGMA_HEAP_CPU_VISIBLE_BIT;
use crate::defines::MAGMA_HEAP_DEVICE_LOCAL_BIT;
use crate::defines::MAGMA_MAX_PHYSICAL_DEVICES;
use crate::defines::MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT;
use crate::defines::MAGMA_MEMORY_PROPERTY_HOST_CACHED_BIT;
use crate::defines::MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT;
use crate::defines::MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT;
use crate::device::PhysicalDevice;
use crate::error::Error;
use crate::error::Result;
use crate::traits::AsVirtGpu;
use crate::traits::BackendBuffer;
use crate::traits::BackendDevice;
use crate::traits::BackendPhysicalDevice;
use crate::traits::BackendQueue;
use crate::traits::GenericDevice;
use crate::traits::GenericPhysicalDevice;

#[allow(non_camel_case_types)]
pub type zx_handle_t = u32;
#[allow(non_camel_case_types)]
pub type zx_status_t = i32;
#[allow(non_camel_case_types)]
pub type zx_time_t = i64;
#[allow(non_camel_case_types)]
pub type zx_duration_t = i64;
#[allow(non_camel_case_types)]
pub type zx_signals_t = u32;

pub const ZX_HANDLE_INVALID: zx_handle_t = 0;
pub const ZX_OK: zx_status_t = 0;
pub const ZX_ERR_INTERNAL: zx_status_t = -1;
pub const ZX_ERR_NOT_SUPPORTED: zx_status_t = -2;
pub const ZX_ERR_NO_RESOURCES: zx_status_t = -3;
pub const ZX_ERR_NO_MEMORY: zx_status_t = -4;
pub const ZX_ERR_INVALID_ARGS: zx_status_t = -10;
pub const ZX_ERR_BAD_HANDLE: zx_status_t = -11;
pub const ZX_ERR_WRONG_TYPE: zx_status_t = -12;
pub const ZX_ERR_BAD_STATE: zx_status_t = -20;
pub const ZX_ERR_TIMED_OUT: zx_status_t = -21;
pub const ZX_ERR_SHOULD_WAIT: zx_status_t = -22;
pub const ZX_ERR_CANCELED: zx_status_t = -23;
pub const ZX_ERR_PEER_CLOSED: zx_status_t = -24;
pub const ZX_ERR_ACCESS_DENIED: zx_status_t = -30;
pub const ZX_ERR_IO: zx_status_t = -40;

pub const ZX_CHANNEL_READABLE: zx_signals_t = 1 << 0;
pub const ZX_CHANNEL_PEER_CLOSED: zx_signals_t = 1 << 2;

pub const MAGMA_QUERY_VENDOR_ID: u64 = 0;
pub const MAGMA_QUERY_DEVICE_ID: u64 = 1;
pub const MAGMA_QUERY_MAXIMUM_INFLIGHT_PARAMS: u64 = 5;
pub const MAGMA_QUERY_VENDOR_PARAM_0: u64 = 10000;
pub const MAGMA_INTEL_GEN_QUERY_GTT_SIZE: u64 = MAGMA_QUERY_VENDOR_PARAM_0 + 1;

const DEVICE_NAMESPACES: [&str; 1] = ["/loader-gpu-devices/svc/magma/"];

const MAX_PATH_LEN: usize = 256;

type OpenInNamespaceCallback = unsafe extern "C" fn(name: *const c_char, handle: zx_handle_t) -> i32;

static OPEN_IN_NAMESPACE_CALLBACK: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

pub fn init_open_in_namespace_callback(callback: *mut c_void) {
    OPEN_IN_NAMESPACE_CALLBACK.store(callback, Ordering::Release);
}

extern "C" {
    fn zx_channel_create(
        options: u32,
        out0: *mut zx_handle_t,
        out1: *mut zx_handle_t,
    ) -> zx_status_t;
    fn zx_handle_close(handle: zx_handle_t) -> zx_status_t;
    fn zx_object_wait_one(
        handle: zx_handle_t,
        signals: zx_signals_t,
        deadline: zx_time_t,
        observed: *mut zx_signals_t,
    ) -> zx_status_t;
    fn zx_deadline_after(nanoseconds: zx_duration_t) -> zx_time_t;
    fn zx_system_get_physmem() -> u64;

    fn magma_fidl_enumerate_devices(
        device_namespace: *const c_char,
        dir_channel: u32,
        max_devices: u32,
        path_size: u32,
        paths_out: *mut c_char,
        count_out: *mut u32,
    ) -> zx_status_t;
    fn magma_fidl_get_current_thread_koid() -> u64;
    fn magma_fidl_device_query(
        device_channel: u32,
        query_id: u64,
        result_buffer_out: *mut u32,
        result_out: *mut u64,
    ) -> zx_status_t;
    fn magma_fidl_device_connect2(
        device_channel: u32,
        client_id: u64,
        primary_channel_out: *mut u32,
        notification_channel_out: *mut u32,
    ) -> zx_status_t;
    fn magma_fidl_primary_enable_flow_control(primary_channel: u32) -> zx_status_t;
    fn magma_fidl_primary_flush(primary_channel: u32) -> zx_status_t;
    fn magma_fidl_primary_handle_one_event(
        primary_channel: u32,
        messages_consumed_out: *mut u64,
        memory_imported_out: *mut u64,
    ) -> zx_status_t;
}

pub fn zx_status_to_result(status: zx_status_t) -> Result<()> {
    match status {
        ZX_OK => Ok(()),
        ZX_ERR_INVALID_ARGS | ZX_ERR_BAD_HANDLE | ZX_ERR_WRONG_TYPE => Err(Error::InvalidArgs),
        ZX_ERR_ACCESS_DENIED => Err(Error::AccessDenied),
        ZX_ERR_NO_MEMORY | ZX_ERR_NO_RESOURCES => Err(Error::MemoryError),
        ZX_ERR_IO | ZX_ERR_PEER_CLOSED | ZX_ERR_CANCELED => Err(Error::ContextKilled),
        ZX_ERR_TIMED_OUT | ZX_ERR_SHOULD_WAIT => Err(Error::TimedOut),
        ZX_ERR_NOT_SUPPORTED => Err(Error::Unimplemented),
        ZX_ERR_INTERNAL | ZX_ERR_BAD_STATE | _ => Err(Error::InternalError),
    }
}

#[derive(Debug)]
pub struct ZxHandle(zx_handle_t);

impl ZxHandle {
    pub fn new(handle: zx_handle_t) -> Self {
        Self(handle)
    }

    pub fn raw(&self) -> zx_handle_t {
        self.0
    }

    pub fn release(mut self) -> zx_handle_t {
        let handle = self.0;
        self.0 = ZX_HANDLE_INVALID;
        handle
    }
}

impl Drop for ZxHandle {
    fn drop(&mut self) {
        if self.0 != ZX_HANDLE_INVALID {
            unsafe {
                zx_handle_close(self.0);
            }
        }
    }
}

fn magma_fuchsia_open(path: *const c_char, channel_out: *mut u32) -> zx_status_t {
    if path.is_null() || channel_out.is_null() {
        return ZX_ERR_INVALID_ARGS;
    }

    let callback_ptr = OPEN_IN_NAMESPACE_CALLBACK.load(Ordering::Acquire);
    if callback_ptr.is_null() {
        return ZX_ERR_NOT_SUPPORTED;
    }

    let mut client_end: zx_handle_t = ZX_HANDLE_INVALID;
    let mut server_end: zx_handle_t = ZX_HANDLE_INVALID;
    let status = unsafe { zx_channel_create(0, &mut client_end, &mut server_end) };
    if status != ZX_OK {
        return status;
    }
    let client_end = ZxHandle::new(client_end);

    let callback: OpenInNamespaceCallback = unsafe { std::mem::transmute(callback_ptr) };
    let res = unsafe { callback(path, server_end) };
    if res != 0 {
        return ZX_ERR_INTERNAL;
    }

    unsafe {
        *channel_out = client_end.release();
    }
    ZX_OK
}

#[derive(Default)]
struct FlowControlState {
    inflight_count: u64,
    inflight_bytes: u64,
}

pub struct PrimaryConnection {
    primary_channel: ZxHandle,
    _notification_channel: ZxHandle,
    max_inflight_messages: u64,
    max_inflight_bytes: u64,
    flow_control_enabled: bool,
    state: Mutex<FlowControlState>,
}

impl PrimaryConnection {
    pub fn new(
        primary_channel: ZxHandle,
        notification_channel: ZxHandle,
        max_inflight_messages: u64,
        max_inflight_bytes: u64,
    ) -> Self {
        let mut flow_control_enabled = false;
        if max_inflight_messages != 0 && max_inflight_bytes != 0 {
            let status = unsafe { magma_fidl_primary_enable_flow_control(primary_channel.raw()) };
            if status == ZX_OK {
                flow_control_enabled = true;
            }
        }
        Self {
            primary_channel,
            _notification_channel: notification_channel,
            max_inflight_messages,
            max_inflight_bytes,
            flow_control_enabled,
            state: Mutex::new(FlowControlState::default()),
        }
    }

    #[allow(dead_code)]
    pub fn primary_raw(&self) -> zx_handle_t {
        self.primary_channel.raw()
    }

    fn should_wait(
        state: &FlowControlState,
        max_inflight_messages: u64,
        max_inflight_bytes: u64,
        new_bytes: u64,
    ) -> (bool, u64, u64) {
        let count = state.inflight_count + 1;
        let bytes = state.inflight_bytes + new_bytes;

        if count > max_inflight_messages {
            return (true, count, bytes);
        }

        if new_bytes != 0 && state.inflight_bytes < max_inflight_bytes / 2 {
            return (false, count, bytes);
        }

        (new_bytes != 0 && bytes > max_inflight_bytes, count, bytes)
    }

    #[allow(dead_code)]
    pub fn with_flow_control<F>(&self, new_bytes: u64, f: F) -> Result<()>
    where
        F: FnOnce(zx_handle_t) -> zx_status_t,
    {
        let mut state = self.state.lock().map_err(|_| Error::InternalError)?;
        self.flow_control(&mut state, new_bytes);
        let status = f(self.primary_channel.raw());
        if status == ZX_OK {
            self.update_flow_control(&mut state, new_bytes);
        }
        zx_status_to_result(status)
    }

    fn flow_control(&self, state: &mut FlowControlState, new_bytes: u64) {
        if !self.flow_control_enabled {
            return;
        }

        let (mut wait, _, _) = Self::should_wait(
            state,
            self.max_inflight_messages,
            self.max_inflight_bytes,
            new_bytes,
        );

        loop {
            let deadline = if wait {
                unsafe { zx_deadline_after(5_000_000_000) }
            } else {
                0
            };

            let mut observed: zx_signals_t = 0;
            let status = unsafe {
                zx_object_wait_one(
                    self.primary_channel.raw(),
                    ZX_CHANNEL_READABLE | ZX_CHANNEL_PEER_CLOSED,
                    deadline,
                    &mut observed,
                )
            };

            if status == ZX_OK {
                if (observed & ZX_CHANNEL_READABLE) != 0 {
                    let mut consumed = 0u64;
                    let mut imported = 0u64;
                    let ev_status = unsafe {
                        magma_fidl_primary_handle_one_event(
                            self.primary_channel.raw(),
                            &mut consumed,
                            &mut imported,
                        )
                    };
                    if ev_status != ZX_OK {
                        return;
                    }
                    state.inflight_count = state.inflight_count.saturating_sub(consumed);
                    state.inflight_bytes = state.inflight_bytes.saturating_sub(imported);
                } else if (observed & ZX_CHANNEL_PEER_CLOSED) != 0 {
                    return;
                }
            } else if status == ZX_ERR_TIMED_OUT {
                if wait {
                    continue;
                }
            } else {
                return;
            }

            (wait, _, _) = Self::should_wait(
                state,
                self.max_inflight_messages,
                self.max_inflight_bytes,
                new_bytes,
            );
            if !wait {
                break;
            }
        }
    }

    fn update_flow_control(&self, state: &mut FlowControlState, new_bytes: u64) {
        if !self.flow_control_enabled {
            return;
        }
        state.inflight_count += 1;
        state.inflight_bytes += new_bytes;
    }

    #[allow(dead_code)]
    pub fn flush(&self) -> Result<()> {
        let status = unsafe { magma_fidl_primary_flush(self.primary_channel.raw()) };
        zx_status_to_result(status)
    }
}

pub trait PlatformDevice {}

pub trait PlatformPhysicalDevice {}

#[derive(Debug)]
pub struct FuchsiaPhysicalDevice {
    device_channel: ZxHandle,
}

impl FuchsiaPhysicalDevice {
    pub fn new(device_channel: ZxHandle) -> Self {
        Self { device_channel }
    }

    pub fn query_simple(&self, query_id: u64) -> Result<u64> {
        let mut result_out = 0u64;
        let status = unsafe {
            magma_fidl_device_query(
                self.device_channel.raw(),
                query_id,
                null_mut(),
                &mut result_out,
            )
        };
        zx_status_to_result(status)?;
        Ok(result_out)
    }

    #[allow(dead_code)]
    pub fn query_buffer(&self, query_id: u64) -> Result<ZxHandle> {
        let mut buffer_out: u32 = ZX_HANDLE_INVALID;
        let status = unsafe {
            magma_fidl_device_query(
                self.device_channel.raw(),
                query_id,
                &mut buffer_out,
                null_mut(),
            )
        };
        zx_status_to_result(status)?;
        Ok(ZxHandle::new(buffer_out))
    }
}

impl PlatformPhysicalDevice for FuchsiaPhysicalDevice {}
impl AsVirtGpu for FuchsiaPhysicalDevice {}
impl BackendPhysicalDevice for FuchsiaPhysicalDevice {}

impl GenericPhysicalDevice for FuchsiaPhysicalDevice {
    fn create_device(
        self: Arc<Self>,
        device_info: &MagmaPhysicalDeviceInfo,
    ) -> Result<Arc<dyn BackendDevice>> {
        let device = FuchsiaDevice::new(self, device_info)?;
        Ok(Arc::new(device))
    }

    fn get_device_info(&self, info: &mut MagmaPhysicalDeviceInfo) -> Result<()> {
        let vendor_id = self.query_simple(MAGMA_QUERY_VENDOR_ID)? as u16;
        let device_id = self.query_simple(MAGMA_QUERY_DEVICE_ID)? as u16;
        eprintln!("get_device_info: vendor_id: 0x{:x}, device_id: 0x{:x}", vendor_id, device_id);

        info.bus_type = MAGMA_BUS_TYPE_PCI;
        info.vendor_id = TryFromBytes::try_read_from_bytes(&vendor_id.to_ne_bytes())
            .map_err(|_| Error::Unimplemented)?;
        info.device_id = device_id;
        Ok(())
    }

    fn get_memory_types(&self) -> Result<Vec<MagmaMemoryType>> {
        Ok(vec![
            MagmaMemoryType {
                property_flags: MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
                    | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT
                    | MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT
                    | MAGMA_MEMORY_PROPERTY_HOST_CACHED_BIT,
                heap_idx: 0,
            },
            MagmaMemoryType {
                property_flags: MAGMA_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
                    | MAGMA_MEMORY_PROPERTY_HOST_VISIBLE_BIT
                    | MAGMA_MEMORY_PROPERTY_HOST_COHERENT_BIT,
                heap_idx: 0,
            },
        ])
    }

    fn get_memory_heaps(&self) -> Result<Vec<MagmaHeap>> {
        let mut heap_size = unsafe { zx_system_get_physmem() };
        if heap_size == 0 {
            heap_size = self
                .query_simple(MAGMA_INTEL_GEN_QUERY_GTT_SIZE)
                .unwrap_or(1u64 << 32);
        }
        Ok(vec![MagmaHeap {
            heap_size,
            heap_flags: MAGMA_HEAP_CPU_VISIBLE_BIT | MAGMA_HEAP_DEVICE_LOCAL_BIT,
        }])
    }

    fn get_queue_family_properties(&self) -> Result<Vec<MagmaQueueFamilyProperties>> {
        Ok(vec![
            MagmaQueueFamilyProperties::new(
                MagmaQueueFlags::Graphics
                    | MagmaQueueFlags::Compute
                    | MagmaQueueFlags::Transfer
                    | MagmaQueueFlags::SparseBinding
                    | MagmaQueueFlags::Protected,
                1,
            ),
            MagmaQueueFamilyProperties::new(
                MagmaQueueFlags::Compute
                    | MagmaQueueFlags::Transfer
                    | MagmaQueueFlags::SparseBinding,
                1,
            ),
            MagmaQueueFamilyProperties::new(
                MagmaQueueFlags::Transfer | MagmaQueueFlags::Protected,
                1,
            ),
        ])
    }
}

pub struct FuchsiaDevice {
    _physical_device: Arc<FuchsiaPhysicalDevice>,
    _connection: Arc<PrimaryConnection>,
    _info: MagmaPhysicalDeviceInfo,
}

impl FuchsiaDevice {
    pub fn new(
        physical_device: Arc<FuchsiaPhysicalDevice>,
        info: &MagmaPhysicalDeviceInfo,
    ) -> Result<Self> {
        let inflight_params = physical_device.query_simple(MAGMA_QUERY_MAXIMUM_INFLIGHT_PARAMS)?;
        let max_inflight_messages = inflight_params >> 32;
        let max_inflight_bytes = (inflight_params & 0xffff_ffff) * 1024 * 1024;

        let client_id = unsafe { magma_fidl_get_current_thread_koid() };
        let mut primary_raw: u32 = ZX_HANDLE_INVALID;
        let mut notification_raw: u32 = ZX_HANDLE_INVALID;
        let status = unsafe {
            magma_fidl_device_connect2(
                physical_device.device_channel.raw(),
                client_id,
                &mut primary_raw,
                &mut notification_raw,
            )
        };
        zx_status_to_result(status)?;

        let connection = Arc::new(PrimaryConnection::new(
            ZxHandle::new(primary_raw),
            ZxHandle::new(notification_raw),
            max_inflight_messages,
            max_inflight_bytes,
        ));

        Ok(Self {
            _physical_device: physical_device,
            _connection: connection,
            _info: *info,
        })
    }
}

impl PlatformDevice for FuchsiaDevice {}
impl BackendDevice for FuchsiaDevice {}

impl GenericDevice for FuchsiaDevice {
    fn create_queue(
        self: Arc<Self>,
        _info: &MagmaCreateQueueInfo,
    ) -> Result<Arc<dyn BackendQueue>> {
        Err(Error::Unimplemented)
    }

    fn create_buffer(
        self: Arc<Self>,
        _create_info: &MagmaCreateBufferInfo,
    ) -> Result<Arc<dyn BackendBuffer>> {
        Err(Error::Unimplemented)
    }

    fn import(self: Arc<Self>, _info: MagmaImportHandleInfo) -> Result<Arc<dyn BackendBuffer>> {
        Err(Error::Unimplemented)
    }
}

fn enumerate_namespace(ns: &str) -> Result<Vec<String>> {
    eprintln!("enumerate_namespace called: {ns}");
    let c_ns = CString::new(ns).map_err(|_| Error::InvalidArgs)?;
    let mut dir_channel: u32 = ZX_HANDLE_INVALID;
    let status = magma_fuchsia_open(c_ns.as_ptr(), &mut dir_channel);
    zx_status_to_result(status)?;

    let max_devices = MAGMA_MAX_PHYSICAL_DEVICES as u32;
    let path_size = MAX_PATH_LEN as u32;
    let mut paths_buf = vec![0u8; (max_devices * path_size) as usize];
    let mut count: u32 = 0;

    let status = unsafe {
        magma_fidl_enumerate_devices(
            c_ns.as_ptr(),
            dir_channel,
            max_devices,
            path_size,
            paths_buf.as_mut_ptr() as *mut c_char,
            &mut count,
        )
    };
    zx_status_to_result(status)?;

    let mut paths = Vec::with_capacity(count as usize);
    for i in 0..(count as usize) {
        let offset = i * MAX_PATH_LEN;
        let slice = &paths_buf[offset..offset + MAX_PATH_LEN];
        if let Ok(c_str) = CStr::from_bytes_until_nul(slice) {
            if let Ok(s) = c_str.to_str() {
                paths.push(s.to_owned());
            }
        }
    }

    Ok(paths)
}

pub fn enumerate_devices() -> Result<Vec<PhysicalDevice>> {
    let mut device_paths = Vec::new();
    for ns in DEVICE_NAMESPACES {
        if let Ok(paths) = enumerate_namespace(ns) {
            if !paths.is_empty() {
                device_paths = paths;
                break;
            }
        }
    }

    let mut devices = Vec::new();
    for path in device_paths {
        let Ok(c_path) = CString::new(path) else {
            continue;
        };
        let mut dev_channel: u32 = ZX_HANDLE_INVALID;
        let status = magma_fuchsia_open(c_path.as_ptr(), &mut dev_channel);
        if status != ZX_OK || dev_channel == ZX_HANDLE_INVALID {
            continue;
        }

        let physical_device = Arc::new(FuchsiaPhysicalDevice::new(ZxHandle::new(dev_channel)));
        let mut info = MagmaPhysicalDeviceInfo::default();
        if physical_device.get_device_info(&mut info).is_err() {
            continue;
        }

        if let Ok(queue_families) = physical_device.get_queue_family_properties() {
            info.queue_family_count = queue_families.len().min(info.queue_families.len()) as u32;
            for (i, qf) in queue_families.iter().enumerate() {
                if i >= info.queue_families.len() {
                    break;
                }
                info.queue_families[i] = *qf;
            }
        }

        devices.push(PhysicalDevice::new(physical_device, info));
    }

    Ok(devices)
}
