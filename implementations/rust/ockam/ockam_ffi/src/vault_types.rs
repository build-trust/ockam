#![allow(conflicting_repr_hints)]

use crate::FfiError;
use ockam_core::lib::convert::TryFrom;
use ockam_vault_core::{SecretAttributes, SecretPersistence, SecretType};

/// Represents a handle id for the secret key
pub type SecretKeyHandle = u64;

#[derive(Clone, Copy, Debug)]
#[repr(C, u8)]
pub enum FfiVaultType {
    Software = 1,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FfiVaultFatPointer {
    handle: u64,
    vault_type: FfiVaultType,
}

impl FfiVaultFatPointer {
    pub fn handle(&self) -> u64 {
        self.handle
    }
    pub fn vault_type(&self) -> FfiVaultType {
        self.vault_type
    }
}

impl FfiVaultFatPointer {
    pub fn new(handle: u64, vault_type: FfiVaultType) -> Self {
        FfiVaultFatPointer { handle, vault_type }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FfiSecretAttributes {
    length: u32,
    stype: u32,
    persistence: u32,
}

impl FfiSecretAttributes {
    pub fn length(&self) -> u32 {
        self.length
    }
    pub fn stype(&self) -> u32 {
        self.stype
    }
    pub fn persistence(&self) -> u32 {
        self.persistence
    }
}

impl FfiSecretAttributes {
    pub fn new(length: u32, stype: u32, persistence: u32) -> Self {
        FfiSecretAttributes {
            length,
            stype,
            persistence,
        }
    }
}

impl From<SecretAttributes> for FfiSecretAttributes {
    fn from(attrs: SecretAttributes) -> Self {
        let stype = match attrs.stype() {
            SecretType::Buffer => 0,
            SecretType::Aes => 1,
            SecretType::Curve25519 => 2,
            SecretType::P256 => 3,
        };

        let persistence = match attrs.persistence() {
            SecretPersistence::Ephemeral => 0,
            SecretPersistence::Persistent => 1,
        };

        Self::new(stype, persistence, attrs.length() as u32)
    }
}

impl TryFrom<FfiSecretAttributes> for SecretAttributes {
    type Error = FfiError;

    fn try_from(attrs: FfiSecretAttributes) -> Result<Self, Self::Error> {
        let stype = match attrs.stype() {
            0 => Ok(SecretType::Buffer),
            1 => Ok(SecretType::Aes),
            2 => Ok(SecretType::Curve25519),
            3 => Ok(SecretType::P256),
            _ => Err(FfiError::InvalidParam),
        }?;

        let persistence = match attrs.persistence() {
            0 => Ok(SecretPersistence::Ephemeral),
            1 => Ok(SecretPersistence::Persistent),
            _ => Err(FfiError::InvalidParam),
        }?;

        Ok(Self::new(stype, persistence, attrs.length() as usize))
    }
}
