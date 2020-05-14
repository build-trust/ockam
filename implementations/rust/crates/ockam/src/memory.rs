use std::alloc::Layout;
use std::mem;
use std::ptr;

use ockam_vault_sys::{ockam_error_t, ockam_memory_dispatch_table_t, ockam_memory_t};

#[repr(C)]
pub(crate) struct RustAlloc {
    inner: ockam_memory_t,
}
impl AsMut<ockam_memory_t> for RustAlloc {
    fn as_mut(&mut self) -> &mut ockam_memory_t {
        &mut self.inner
    }
}
impl RustAlloc {
    const DISPATCH: ockam_memory_dispatch_table_t = ockam_memory_dispatch_table_t {
        deinit: None,
        alloc_zeroed: Some(self::alloc_zeroed_impl),
        free: Some(self::free_impl),
        set: Some(self::memset_impl),
        copy: Some(self::memcpy_impl),
        move_: Some(self::memmove_impl),
    };

    const GLOBAL: Self = Self {
        inner: ockam_memory_t {
            dispatch: &Self::DISPATCH as *const _ as *mut _,
            context: ptr::null_mut(),
        },
    };

    #[inline(always)]
    pub const fn new() -> &'static Self {
        &Self::GLOBAL
    }
}

unsafe extern "C" fn alloc_zeroed_impl(
    _: *mut ockam_memory_t,
    buffer: *mut *mut core::ffi::c_void,
    size: usize,
) -> ockam_error_t {
    let layout_result = Layout::from_size_align(size, mem::align_of::<core::ffi::c_void>());
    if let Ok(layout) = layout_result {
        let ptr = std::alloc::alloc_zeroed(layout);
        if !ptr.is_null() {
            buffer.write(ptr as *mut _);
            return ockam_vault_sys::OCKAM_ERROR_NONE;
        }
    }

    ockam_vault_sys::OCKAM_MEMORY_ERROR_ALLOC_FAIL
}

unsafe extern "C" fn free_impl(_: *mut ockam_memory_t, ptr: *mut core::ffi::c_void, size: usize) -> ockam_error_t {
    let layout_result = Layout::from_size_align(size, mem::align_of::<core::ffi::c_void>());
    if let Ok(layout) = layout_result {
        std::alloc::dealloc(ptr as *mut _, layout);
        ockam_vault_sys::OCKAM_ERROR_NONE
    } else {
        ockam_vault_sys::OCKAM_MEMORY_ERROR_INVALID_PARAM
    }
}

unsafe extern "C" fn memset_impl(
    _: *mut ockam_memory_t,
    ptr: *mut core::ffi::c_void,
    byte: u8,
    count: usize,
) -> ockam_error_t {
    core::intrinsics::write_bytes(ptr, byte, count);
    ockam_vault_sys::OCKAM_ERROR_NONE
}

unsafe extern "C" fn memcpy_impl(
    _: *mut ockam_memory_t,
    dst: *mut core::ffi::c_void,
    src: *const core::ffi::c_void,
    size: usize,
) -> ockam_error_t {
    core::intrinsics::copy_nonoverlapping(src, dst, size);
    ockam_vault_sys::OCKAM_ERROR_NONE
}

unsafe extern "C" fn memmove_impl(
    _: *mut ockam_memory_t,
    dst: *mut core::ffi::c_void,
    src: *mut core::ffi::c_void,
    size: usize,
) -> ockam_error_t {
    core::intrinsics::copy(src, dst, size);
    ockam_vault_sys::OCKAM_ERROR_NONE
}
