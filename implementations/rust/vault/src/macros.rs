macro_rules! try_from_int_impl {
    ($src:ident, $ty:ty) => {
        impl std::convert::TryFrom<$ty> for $src {
            type Error = VaultFailError;

            fn try_from(value: $ty) -> Result<Self, Self::Error> {
                Self::from_usize(value as usize)
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
    };
}

macro_rules! fail {
    ($err:expr) => {
        return Err($err.into());
    };
}
