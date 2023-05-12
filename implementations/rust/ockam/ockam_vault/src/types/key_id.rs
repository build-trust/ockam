use cfg_if::cfg_if;
use ockam_core::compat::string::String;

cfg_if! {
    if #[cfg(not(feature = "alloc"))] {
        /// ID of a Key.
        pub type KeyId = heapless::String<64>;

        impl From<&str> for KeyId {
            fn from(s: &str) -> Self {
                heapless::String::from(s)
            }
        }
    }
    else {
        /// ID of a Key.
        pub type KeyId = String;
    }
}
