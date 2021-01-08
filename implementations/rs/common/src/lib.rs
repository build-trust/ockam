#![deny(missing_docs)]

//! Common macros and error

/// From int impl
#[macro_export]
macro_rules! from_int_impl {
    ($src:ident, $ty:ty) => {
        impl From<$src> for $ty {
            fn from(data: $src) -> $ty {
                data.to_usize() as $ty
            }
        }
    };
}

/// Fail impl
#[macro_export]
macro_rules! fail {
    ($err:expr) => {
        return Err($err.into());
    };
}

/// Creates drop implementation with zeroize call
#[macro_export]
macro_rules! zdrop_impl {
    ($name:ident) => {
        impl Drop for $name {
            fn drop(&mut self) {
                self.zeroize();
            }
        }
    };
}

/// Error
pub mod error;
