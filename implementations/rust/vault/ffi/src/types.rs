use ffi_support::IntoFfi;
use ockam_vault_software::ockam_vault::types::*;
use std::convert::TryInto;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FfiSecretAttributes {
    pub(crate) length: u32,
    pub(crate) xtype: u32,
    pub(crate) persistence: u32,
}

impl From<SecretAttributes> for FfiSecretAttributes {
    fn from(attrs: SecretAttributes) -> Self {
        Self {
            length: attrs.length as u32,
            xtype: attrs.stype.to_usize() as u32,
            persistence: attrs.persistence.to_usize() as u32,
        }
    }
}

impl From<FfiSecretAttributes> for SecretAttributes {
    fn from(attrs: FfiSecretAttributes) -> Self {
        Self::from(&attrs)
    }
}

impl From<&FfiSecretAttributes> for SecretAttributes {
    fn from(attrs: &FfiSecretAttributes) -> Self {
        Self {
            stype: attrs.xtype.try_into().unwrap(),
            persistence: attrs.persistence.try_into().unwrap(),
            length: attrs.length as usize,
        }
    }
}

unsafe impl IntoFfi for FfiSecretAttributes {
    type Value = FfiSecretAttributes;

    fn ffi_default() -> Self::Value {
        Self {
            length: 0,
            xtype: 0,
            persistence: 0,
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
/// Error or success
pub const ERROR: u32 = 1;

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
