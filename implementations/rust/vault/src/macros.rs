
#[cfg(not(feature = "nostd-stm32f4"))]
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

#[cfg(feature = "nostd-stm32f4")]
macro_rules! try_from_int_impl {
    ($src:ident, $ty:ty) => {
        // !from_int_impl($ty)
    };
}

/*
 * This is really the only needed item from ockam_common,
 * and ockam_common pulls in ockam_message, which pulls in
 * all sorts of std support items, so for now it's duplicated here
 * and the ockam_common dependency for vault is removed
 */
// #[cfg(feature = "nostd-stm32f4")]
macro_rules! from_int_impl {
    ($src:ident, $ty:ty) => {
        impl From<$src> for $ty {
            fn from(data: $src) -> $ty {
                data.to_usize() as $ty
            }
        }
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
