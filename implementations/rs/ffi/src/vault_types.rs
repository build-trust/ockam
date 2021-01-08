#![allow(conflicting_repr_hints)]

use ockam_vault_software::ockam_vault::types::*;
use std::convert::TryInto;

#[derive(Clone, Copy, Debug)]
#[repr(C, u8)]
pub enum FfiVaultType {
    Software = 1,
    Filesystem = 2,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FfiVaultFatPointer {
    pub(crate) handle: u64,
    pub(crate) vault_type: FfiVaultType,
}

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

/// Represents a handle id for the secret key
pub type SecretKeyHandle = u64;
