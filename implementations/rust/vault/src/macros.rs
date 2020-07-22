macro_rules! from_int_impl {
    ($src:ident, $ty:ty) => {
        impl From<$src> for $ty {
            fn from(data: $src) -> $ty {
                data.to_usize() as $ty
            }
        }
    };
}

macro_rules! zdrop_impl {
    ($name:ident) => {
        impl Drop for $name {
            fn drop(&mut self) {
                self.zeroize();
            }
        }
    };
}

#[cfg(feature = "ffi")]
macro_rules! check_buffer {
    ($buffer:expr) => {
        if $buffer.is_null() {
            return VaultFailErrorKind::InvalidParam(1).into();
        }
    };
    ($buffer:expr, $length:expr) => {
        if $buffer.is_null() {
            return VaultFailErrorKind::InvalidParam(1).into();
        }
        if $length == 0 {
            return VaultFailErrorKind::InvalidParam(2).into();
        }
    }
}
