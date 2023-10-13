mod functions;
mod mock_data;
pub mod notification;
pub mod state;

use libc::c_char;
use std::ffi::CString;
use std::ptr::null;

/// Every string created this way must be manually freed
pub fn to_optional_c_string(opt_s: Option<String>) -> *const c_char {
    match opt_s {
        Some(s) => CString::new(s).unwrap().into_raw(),
        None => std::ptr::null(),
    }
}

/// Every string created this way must be manually freed
pub fn to_c_string(s: String) -> *const c_char {
    CString::new(s).unwrap().into_raw()
}

/// Appends the last null pointer to represent the end of the list and convert the iterator
/// into a raw array pointer
pub fn append_c_terminator<T>(vec: Vec<*const T>) -> *const *const T {
    let mut vec = vec;
    vec.push(null());
    Box::into_raw(vec.into_boxed_slice()) as *const *const T
}
