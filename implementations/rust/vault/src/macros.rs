macro_rules! from_int_impl {
    ($src:ident, $ty:ty) => {
        impl From<$src> for $ty {
            fn from(data: $src) -> $ty {
                data.to_usize() as $ty
            }
        }
    };
}
