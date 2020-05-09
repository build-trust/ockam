#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

impl Into<u32> for VaultFeatures {
    #[inline(always)]
    fn into(self) -> u32 {
        self.0
    }
}
