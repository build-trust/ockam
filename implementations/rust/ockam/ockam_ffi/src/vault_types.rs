#![allow(conflicting_repr_hints)]

use crate::FfiError;
use ockam_vault::constants::AES256_SECRET_LENGTH_U32;
use ockam_vault::{SecretAttributes, SecretType};

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
    length: u32,
}

impl FfiSecretAttributes {
    pub fn stype(&self) -> u8 {
        self.stype
    }
    pub fn length(&self) -> u32 {
        self.length
    }
}

impl FfiSecretAttributes {
    pub fn new(stype: u8, length: u32) -> Self {
        Self { stype, length }
    }
}

impl From<SecretAttributes> for FfiSecretAttributes {
    fn from(attrs: SecretAttributes) -> Self {
        let stype = match attrs.secret_type() {
            SecretType::Buffer => 0,
            SecretType::Aes => 1,
            SecretType::X25519 => 2,
            SecretType::Ed25519 => 3,
            SecretType::NistP256 => 4,
        };

        Self::new(stype, attrs.length())
    }
}

impl TryFrom<FfiSecretAttributes> for SecretAttributes {
    type Error = FfiError;

    fn try_from(attrs: FfiSecretAttributes) -> Result<Self, Self::Error> {
        match attrs.stype() {
            0 => Ok(SecretAttributes::Buffer(attrs.length)),
            1 => Ok(if attrs.length == AES256_SECRET_LENGTH_U32 {
                SecretAttributes::Aes256
            } else {
                SecretAttributes::Aes128
            }),
            2 => Ok(SecretAttributes::X25519),
            3 => Ok(SecretAttributes::Ed25519),
            _ => Err(FfiError::InvalidParam),
        }
    }
}
