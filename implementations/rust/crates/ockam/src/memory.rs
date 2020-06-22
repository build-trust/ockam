use std::alloc::Layout;
use std::mem;
use std::ptr;
use libc::memcmp;

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
        compare: Some(self::memcmp_impl),
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

    #[inline(always)]
    pub const fn as_mut_ptr(&self) -> *mut ockam_memory_t {
        self as *const _ as *mut ockam_memory_t
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

unsafe extern "C" fn free_impl(
    _: *mut ockam_memory_t,
    ptr: *mut core::ffi::c_void,
    size: usize,
) -> ockam_error_t {
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

unsafe extern "C" fn memcmp_impl(
    _: *mut ockam_memory_t,
    res: *mut i32,
    lhs: *const core::ffi::c_void,
    rhs: *const core::ffi::c_void,
    size: usize,
) -> ockam_error_t {
    if res.is_null() || lhs.is_null() || rhs.is_null() {
        return ockam_vault_sys::OCKAM_MEMORY_ERROR_INVALID_PARAM;
    }

    *res = memcmp(lhs, rhs, size);

    ockam_vault_sys::OCKAM_ERROR_NONE
}

#[cfg(test)]
mod tests {
    use crate::memory::{memcmp_impl, RustAlloc};
    use core::ffi::c_void;

    #[test]
    fn cmp() {
        #[allow(improper_ctypes)]
        let block1: [u8; 5] = [0x01, 0x02, 0x03, 0x04, 0x04];
        let block2: [u8; 5] = [0x01, 0x02, 0x03, 0x04, 0x05];
        let block3: [u8; 5] = [0x01, 0x02, 0x03, 0x04, 0x05];
        let block4: [u8; 5] = [0x01, 0x02, 0x03, 0x04, 0x06];

        let mem = RustAlloc::new().as_mut_ptr();

        unsafe {
            let mut res: i32 = 2;
            let err = memcmp_impl(
                mem,
                &mut res,
                block2.as_ptr() as *const c_void,
                block1.as_ptr() as *const c_void,
                block2.len(),
            );
            assert_eq!(err, ockam_vault_sys::OCKAM_ERROR_NONE);
            assert_eq!(res, 1);

            let mut res: i32 = 2;
            let err = memcmp_impl(
                mem,
                &mut res,
                block2.as_ptr() as *const c_void,
                block3.as_ptr() as *const c_void,
                block2.len(),
            );
            assert_eq!(err, ockam_vault_sys::OCKAM_ERROR_NONE);
            assert_eq!(res, 0);

            let mut res: i32 = 2;
            let err = memcmp_impl(
                mem,
                &mut res,
                block2.as_ptr() as *const c_void,
                block4.as_ptr() as *const c_void,
                block2.len(),
            );
            assert_eq!(err, ockam_vault_sys::OCKAM_ERROR_NONE);
            assert_eq!(res, -1);
        }
    }
}
