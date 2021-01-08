#![allow(conflicting_repr_hints)]

#[derive(Clone, Copy, Debug)]
#[repr(C, u8)]
pub enum FfiKexType {
    XxInitiator = 1,
    XxResponder = 2,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FfiKexFatPointer {
    pub(crate) handle: u64,
    pub(crate) kex_type: FfiKexType,
}

/// A Completed Key Exchange elements
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FfiCompletedKeyExchange {
    /// The state hash
    pub h: [u8; 32],
    /// The derived encryption key handle
    pub encrypt_key: u64,
    /// The derived decryption key handle
    pub decrypt_key: u64,
    /// The long term static public key from remote party
    pub remote_static_public_key: [u8; 65],
    /// The long term static public key len
    pub remote_static_public_key_len: usize,
}
