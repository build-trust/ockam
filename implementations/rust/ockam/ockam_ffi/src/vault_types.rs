#![allow(conflicting_repr_hints)]

use crate::FfiError;
use ockam_core::vault::{KeyAttributes, KeyPersistence, KeyType};

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

impl From<KeyAttributes> for FfiSecretAttributes {
    fn from(attrs: KeyAttributes) -> Self {
        let stype = match attrs.stype() {
            KeyType::Buffer => 0,
            KeyType::Aes => 1,
            KeyType::X25519 => 2,
            KeyType::Ed25519 => 3,
            KeyType::NistP256 => 4,
        };

        let persistence = match attrs.persistence() {
            KeyPersistence::Ephemeral => 0,
            KeyPersistence::Persistent => 1,
        };

        Self::new(stype, persistence, attrs.length())
    }
}

impl TryFrom<FfiSecretAttributes> for KeyAttributes {
    type Error = FfiError;

    fn try_from(attrs: FfiSecretAttributes) -> Result<Self, Self::Error> {
        let stype = match attrs.stype() {
            0 => Ok(KeyType::Buffer),
            1 => Ok(KeyType::Aes),
            2 => Ok(KeyType::X25519),
            3 => Ok(KeyType::Ed25519),
            _ => Err(FfiError::InvalidParam),
        }?;

        let persistence = match attrs.persistence() {
            0 => Ok(KeyPersistence::Ephemeral),
            1 => Ok(KeyPersistence::Persistent),
            _ => Err(FfiError::InvalidParam),
        }?;

        Ok(Self::new(stype, persistence, attrs.length()))
    }
}
