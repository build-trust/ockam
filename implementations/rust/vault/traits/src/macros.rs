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
