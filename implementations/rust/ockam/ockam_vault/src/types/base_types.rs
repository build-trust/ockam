use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(not(feature = "alloc"))] {
        /// Secret Key Vector. The maximum size is 32 bytes.
        pub type SecretKeyVec = heapless::Vec<u8, 32>;
        /// Public Key Vector. The maximum size is 65 bytes.
        pub type PublicKeyVec = heapless::Vec<u8, 65>;
        /// Buffer for small vectors (e.g. an array of attributes). The maximum length is 4 elements.
        pub type SmallBuffer<T> = heapless::Vec<T, 4>;
        /// Buffer for large binaries (e.g. encrypted data). The maximum length is 512 elements.
        pub type Buffer<T> = heapless::Vec<T, 512>;
        /// Signature Vector. The maximum size is 112 bytes.
        pub type SignatureVec = heapless::Vec<u8, 112>;

        impl From<&str> for KeyId {
            fn from(s: &str) -> Self {
                heapless::String::from(s)
            }
        }
    }
    else {
        use alloc::vec::Vec;
        /// Secret Key Vector.
        pub type SecretKeyVec = Vec<u8>;
        /// Public Key Vector.
        pub type PublicKeyVec = Vec<u8>;
        /// Buffer for small vectors. (e.g. an array of attributes)
        pub type SmallBuffer<T> = Vec<T>;
        /// Buffer for large binaries. (e.g. encrypted data)
        pub type Buffer<T> = Vec<T>;
        /// Signature Vector.
        pub type SignatureVec = Vec<u8>;
    }
}
