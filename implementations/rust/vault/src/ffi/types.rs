use crate::error::VaultFailError;
use crate::types::*;
use ffi_support::IntoFfi;
use std::convert::{TryFrom, TryInto};

#[derive(Debug)]
#[repr(C)]
pub struct FfiSecretKey {
    pub(crate) length: u32,
    pub(crate) xtype: u32,
    pub(crate) buffer: *mut u8,
}

impl FfiSecretKey {
    pub fn to_vec(&self) -> Vec<u8> {
        if self.buffer.is_null() {
            vec![]
        } else {
            unsafe {
                let mut dst = Vec::with_capacity(self.length as usize);
                dst.set_len(self.length as usize);
                std::ptr::copy(self.buffer, dst.as_mut_ptr(), self.length as usize);
                dst
            }
        }
    }
}

impl From<SecretKey> for FfiSecretKey {
    fn from(sk: SecretKey) -> Self {
        let (xtype, length, mut buf) = match &sk {
            SecretKey::Buffer(a) => {
                let buf = a.to_vec().into_boxed_slice();
                (SecretKeyType::Buffer(buf.len()), buf.len() as u32, buf)
            }
            SecretKey::Aes128(a) => {
                let buf = a.to_vec().into_boxed_slice();
                (SecretKeyType::Aes128, buf.len() as u32, buf)
            }
            SecretKey::Aes256(a) => {
                let buf = a.to_vec().into_boxed_slice();
                (SecretKeyType::Aes256, buf.len() as u32, buf)
            }
            SecretKey::Curve25519(a) => {
                let buf = a.to_vec().into_boxed_slice();
                (SecretKeyType::Curve25519, buf.len() as u32, buf)
            }
            SecretKey::P256(a) => {
                let buf = a.to_vec().into_boxed_slice();
                (SecretKeyType::P256, buf.len() as u32, buf)
            }
        };
        let s = FfiSecretKey {
            xtype: xtype.into(),
            length,
            buffer: buf.as_mut_ptr(),
        };
        std::mem::forget(buf);
        s
    }
}

impl From<FfiSecretKey> for Vec<u8> {
    fn from(key: FfiSecretKey) -> Vec<u8> {
        key.to_vec()
    }
}

impl TryFrom<FfiSecretKey> for SecretKey {
    type Error = VaultFailError;

    fn try_from(key: FfiSecretKey) -> Result<Self, Self::Error> {
        let a = key.to_vec();
        let s = match SecretKeyType::from_usize(key.xtype as usize)? {
            SecretKeyType::Buffer(_) => SecretKey::Buffer(a),
            SecretKeyType::Aes128 => SecretKey::Aes128(*array_ref![a, 0, 16]),
            SecretKeyType::Aes256 => SecretKey::Aes256(*array_ref![a, 0, 32]),
            SecretKeyType::P256 => SecretKey::P256(*array_ref![a, 0, 32]),
            SecretKeyType::Curve25519 => SecretKey::Curve25519(*array_ref![a, 0, 32]),
        };
        Ok(s)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FfiSecretKeyAttributes {
    pub(crate) xtype: u32,
    pub(crate) persistence: u32,
    pub(crate) purpose: u32,
}

impl From<SecretKeyAttributes> for FfiSecretKeyAttributes {
    fn from(attrs: SecretKeyAttributes) -> Self {
        Self {
            xtype: attrs.xtype.to_usize() as u32,
            persistence: attrs.persistence.to_usize() as u32,
            purpose: attrs.purpose.to_usize() as u32,
        }
    }
}

impl From<FfiSecretKeyAttributes> for SecretKeyAttributes {
    fn from(attrs: FfiSecretKeyAttributes) -> Self {
        Self::from(&attrs)
    }
}

impl From<&FfiSecretKeyAttributes> for SecretKeyAttributes {
    fn from(attrs: &FfiSecretKeyAttributes) -> Self {
        Self {
            xtype: attrs.xtype.try_into().unwrap(),
            persistence: attrs.persistence.try_into().unwrap(),
            purpose: attrs.purpose.try_into().unwrap(),
        }
    }
}

unsafe impl IntoFfi for FfiSecretKeyAttributes {
    type Value = FfiSecretKeyAttributes;

    fn ffi_default() -> Self::Value {
        Self {
            xtype: 0,
            persistence: 0,
            purpose: 0,
        }
    }

    fn into_ffi_value(self) -> Self::Value {
        self
    }
}

/// Represents a Vault id
pub type VaultId = u32;
/// Represents a Vault handle
pub type VaultHandle = u64;
/// Represents a Vault error code
pub type VaultError = u32;
/// Represents a handle id for the secret key
pub type SecretKeyHandle = u64;
/// No error or success
pub const ERROR_NONE: u32 = 0;

/// A context object to interface with C
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct OckamVaultContext {
    pub(crate) handle: VaultHandle,
    pub(crate) vault_id: VaultId,
}

pub struct OckamSecretList(pub(crate) Vec<u64>);

unsafe impl IntoFfi for OckamSecretList {
    type Value = Vec<u64>;

    fn ffi_default() -> Self::Value {
        Vec::new()
    }

    fn into_ffi_value(self) -> Self::Value {
        self.0
    }
}
