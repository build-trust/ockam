#![allow(conflicting_repr_hints)]

use crate::FfiError;
use ockam_core::vault::{SecretAttributes, SecretPersistence, SecretType};

/// Represents a handle id for the secret key
pub type SecretKeyHandle = u64;

#[repr(C, u8)]
#[derive(Clone, Copy, Debug)]
pub enum FfiVaultType {
    Software = 1,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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
    stype: u8,
    persistence: u8,
    length: u32,
}

impl FfiSecretAttributes {
    pub fn stype(&self) -> u8 {
        self.stype
    }
    pub fn persistence(&self) -> u8 {
        self.persistence
    }
    pub fn length(&self) -> u32 {
        self.length
    }
}

impl FfiSecretAttributes {
    pub fn new(stype: u8, persistence: u8, length: u32) -> Self {
        Self {
            stype,
            persistence,
            length,
        }
    }
}

impl From<SecretAttributes> for FfiSecretAttributes {
    fn from(attrs: SecretAttributes) -> Self {
        let stype = match attrs.stype() {
            SecretType::Buffer => 0,
            SecretType::Aes => 1,
            SecretType::X25519 => 2,
            SecretType::Ed25519 => 3,
            #[cfg(feature = "bls")]
            SecretType::Bls => 4,
        };

        let persistence = match attrs.persistence() {
            SecretPersistence::Ephemeral => 0,
            SecretPersistence::Persistent => 1,
        };

        Self::new(stype, persistence, attrs.length())
    }
}

impl TryFrom<FfiSecretAttributes> for SecretAttributes {
    type Error = FfiError;

    fn try_from(attrs: FfiSecretAttributes) -> Result<Self, Self::Error> {
        let stype = match attrs.stype() {
            0 => Ok(SecretType::Buffer),
            1 => Ok(SecretType::Aes),
            2 => Ok(SecretType::X25519),
            3 => Ok(SecretType::Ed25519),
            #[cfg(feature = "bls")]
            4 => Ok(SecretType::Bls),
            _ => Err(FfiError::InvalidParam),
        }?;

        let persistence = match attrs.persistence() {
            0 => Ok(SecretPersistence::Ephemeral),
            1 => Ok(SecretPersistence::Persistent),
            _ => Err(FfiError::InvalidParam),
        }?;

        Ok(Self::new(stype, persistence, attrs.length()))
    }
}
