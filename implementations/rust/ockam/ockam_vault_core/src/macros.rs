/// Create drop implementation with zeroize call
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
