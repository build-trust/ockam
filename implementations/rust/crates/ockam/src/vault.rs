#[cfg(test)]
mod test;

use std::convert::TryInto;
use std::convert::{AsMut, AsRef};
use std::mem::{self, MaybeUninit};
use std::ptr;

use cfg_if::cfg_if;
use thiserror::Error;

pub use ockam_vault_sys::VaultFeatures;
use ockam_vault_sys::{ockam_error_t, ockam_random_t};
use ockam_vault_sys::{ockam_vault_default_attributes_t, ockam_vault_t};
use ockam_vault_sys::{
    ockam_vault_secret_attributes_t, ockam_vault_secret_persistence_t,
    ockam_vault_secret_purpose_t, ockam_vault_secret_t, ockam_vault_secret_type_t,
};

use crate::memory::RustAlloc;

cfg_if! {
    if #[cfg(feature = "term_encoding")] {
        use rustler;
        use rustler::{NifUnitEnum, NifStruct};
    }
}

pub type VaultResult<T> = Result<T, VaultError>;

#[derive(Error, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VaultError {
    #[error("invalid_parameter")]
    InvalidParameter,
    #[error("invalid_attributes")]
    InvalidAttributes,
    #[error("invalid_context")]
    InvalidContext,
    #[error("invalid_buffer")]
    InvalidBuffer,
    #[error("invalid_size")]
    InvalidSize,
    #[error("buffer_too_small")]
    BufferTooSmall,
    #[error("invalid_regenerate")]
    InvalidRegenerate,
    #[error("invalid_secret_attributes")]
    InvalidSecretAttributes,
    #[error("invalid_secret_type")]
    InvalidSecretType,
    #[error("invalid_tag")]
    InvalidTag,
    #[error("publickey_error")]
    PublicKeyError,
    #[error("ecdh_error")]
    EcdhError,
    #[error("default_random_required")]
    DefaultRandomRequired,
    #[error("memory_required")]
    MemoryRequired,
    #[error("secret_size_mismatch")]
    SecretSizeMismatch,
    #[error("keygen_error")]
    KeyGenError,
    #[error("failed")]
    Unknown,
}
impl VaultError {
    #[inline]
    fn wrap<F>(mut fun: F) -> VaultResult<()>
    where
        F: FnMut() -> ockam_error_t,
    {
        match fun() {
            ockam_vault_sys::OCKAM_ERROR_NONE => Ok(()),
            code => Err(code.into()),
        }
    }
}
impl From<ockam_error_t> for VaultError {
    fn from(err: ockam_error_t) -> Self {
        assert_ne!(
            err,
            ockam_vault_sys::OCKAM_ERROR_NONE,
            "expected error, but got OCKAM_ERROR_NONE"
        );
        match err {
            ockam_vault_sys::OCKAM_VAULT_ERROR_INVALID_PARAM => Self::InvalidParameter,
            ockam_vault_sys::OCKAM_VAULT_ERROR_INVALID_ATTRIBUTES => Self::InvalidAttributes,
            ockam_vault_sys::OCKAM_VAULT_ERROR_INVALID_CONTEXT => Self::InvalidContext,
            ockam_vault_sys::OCKAM_VAULT_ERROR_INVALID_BUFFER => Self::InvalidBuffer,
            ockam_vault_sys::OCKAM_VAULT_ERROR_INVALID_SIZE => Self::InvalidSize,
            ockam_vault_sys::OCKAM_VAULT_ERROR_BUFFER_TOO_SMALL => Self::BufferTooSmall,
            ockam_vault_sys::OCKAM_VAULT_ERROR_INVALID_REGENERATE => Self::InvalidRegenerate,
            ockam_vault_sys::OCKAM_VAULT_ERROR_INVALID_SECRET_ATTRIBUTES => {
                Self::InvalidSecretAttributes
            }
            ockam_vault_sys::OCKAM_VAULT_ERROR_INVALID_SECRET_TYPE => Self::InvalidSecretType,
            ockam_vault_sys::OCKAM_VAULT_ERROR_INVALID_TAG => Self::InvalidTag,
            ockam_vault_sys::OCKAM_VAULT_ERROR_PUBLIC_KEY_FAIL => Self::PublicKeyError,
            ockam_vault_sys::OCKAM_VAULT_ERROR_ECDH_FAIL => Self::EcdhError,
            ockam_vault_sys::OCKAM_VAULT_ERROR_DEFAULT_RANDOM_REQUIRED => {
                Self::DefaultRandomRequired
            }
            ockam_vault_sys::OCKAM_VAULT_ERROR_MEMORY_REQUIRED => Self::MemoryRequired,
            ockam_vault_sys::OCKAM_VAULT_ERROR_SECRET_SIZE_MISMATCH => Self::SecretSizeMismatch,
            ockam_vault_sys::OCKAM_VAULT_ERROR_KEYGEN_FAIL => Self::KeyGenError,
            _code => Self::Unknown,
        }
    }
}

#[cfg(feature = "term_encoding")]
rustler::atoms! {
    ok,
    invalid_parameter,
    invalid_attributes,
    invalid_context,
    invalid_buffer,
    invalid_size,
    buffer_too_small,
    invalid_regenerate,
    invalid_secret_attributes,
    invalid_secret_type,
    invalid_tag,
    publickey_error,
    ecdh_error,
    default_random_required,
    memory_required,
    secret_size_mismatch,
    keygen_error,
    failed,
}

#[cfg(feature = "term_encoding")]
impl rustler::Encoder for VaultError {
    fn encode<'c>(&self, env: rustler::Env<'c>) -> rustler::Term<'c> {
        use rustler::Atom;
        let string = self.to_string();
        let atom = Atom::from_bytes(env, string.as_bytes()).unwrap();
        atom.to_term(env)
    }
}

/// Represents the level of persistence a secret is created with
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "term_encoding", derive(NifUnitEnum))]
pub enum SecretPersistence {
    Ephemeral = 0,
    Static,
}
impl From<ockam_vault_secret_persistence_t> for SecretPersistence {
    fn from(value: ockam_vault_secret_persistence_t) -> Self {
        use ockam_vault_secret_persistence_t::*;
        match value {
            OCKAM_VAULT_SECRET_PERSISTENT => Self::Static,
            OCKAM_VAULT_SECRET_EPHEMERAL => Self::Ephemeral,
        }
    }
}
impl Into<ockam_vault_secret_persistence_t> for SecretPersistence {
    fn into(self) -> ockam_vault_secret_persistence_t {
        use ockam_vault_secret_persistence_t::*;
        match self {
            Self::Static => OCKAM_VAULT_SECRET_PERSISTENT,
            Self::Ephemeral => OCKAM_VAULT_SECRET_EPHEMERAL,
        }
    }
}

/// Represents the type of a secret
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "term_encoding", derive(NifUnitEnum))]
pub enum SecretType {
    Buffer = 0,
    AES128,
    AES256,
    Curve25519Private,
    P256Private,
}
impl From<ockam_vault_secret_type_t> for SecretType {
    fn from(value: ockam_vault_secret_type_t) -> Self {
        use ockam_vault_secret_type_t::*;
        match value {
            OCKAM_VAULT_SECRET_TYPE_BUFFER => Self::Buffer,
            OCKAM_VAULT_SECRET_TYPE_AES128_KEY => Self::AES128,
            OCKAM_VAULT_SECRET_TYPE_AES256_KEY => Self::AES256,
            OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY => Self::Curve25519Private,
            OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY => Self::P256Private,
        }
    }
}
impl Into<ockam_vault_secret_type_t> for SecretType {
    fn into(self) -> ockam_vault_secret_type_t {
        use ockam_vault_secret_type_t::*;
        match self {
            Self::Buffer => OCKAM_VAULT_SECRET_TYPE_BUFFER,
            Self::AES128 => OCKAM_VAULT_SECRET_TYPE_AES128_KEY,
            Self::AES256 => OCKAM_VAULT_SECRET_TYPE_AES256_KEY,
            Self::Curve25519Private => OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY,
            Self::P256Private => OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY,
        }
    }
}

/// Represents the purpose for which a given secret is intended to be used
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "term_encoding", derive(NifUnitEnum))]
pub enum SecretPurpose {
    KeyAgreement = 0,
}
impl From<ockam_vault_secret_purpose_t> for SecretPurpose {
    fn from(value: ockam_vault_secret_purpose_t) -> Self {
        use ockam_vault_secret_purpose_t::*;
        match value {
            OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT => Self::KeyAgreement,
        }
    }
}

/// Represents the attributes of a secret
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "term_encoding", derive(NifStruct))]
#[cfg_attr(feature = "term_encoding", module = "Ockam.Vault.SecretAttributes")]
pub struct SecretAttributes {
    pub length: u16,
    pub ty: SecretType,
    pub purpose: SecretPurpose,
    pub persistence: SecretPersistence,
}
impl AsRef<ockam_vault_secret_attributes_t> for SecretAttributes {
    fn as_ref(&self) -> &ockam_vault_secret_attributes_t {
        unsafe { mem::transmute::<&Self, &ockam_vault_secret_attributes_t>(self) }
    }
}
impl AsMut<ockam_vault_secret_attributes_t> for SecretAttributes {
    fn as_mut(&mut self) -> &mut ockam_vault_secret_attributes_t {
        unsafe { mem::transmute::<&mut Self, &mut ockam_vault_secret_attributes_t>(self) }
    }
}
impl Default for SecretAttributes {
    fn default() -> Self {
        Self {
            length: 0,
            ty: SecretType::Buffer,
            purpose: SecretPurpose::KeyAgreement,
            persistence: SecretPersistence::Ephemeral,
        }
    }
}

/// Represents an opaque secret produced from a specific Vault instance
#[repr(C)]
#[derive(Debug)]
pub struct Secret(ockam_vault_secret_t);
impl AsRef<ockam_vault_secret_t> for Secret {
    fn as_ref(&self) -> &ockam_vault_secret_t {
        &self.0
    }
}
impl AsMut<ockam_vault_secret_t> for Secret {
    fn as_mut(&mut self) -> &mut ockam_vault_secret_t {
        &mut self.0
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Random(ockam_random_t);
impl Random {
    pub fn new() -> VaultResult<Self> {
        use ockam_vault_sys::ockam_random_urandom_init;

        let mut context = ockam_random_t {
            dispatch: ptr::null_mut(),
            context: ptr::null_mut(),
        };

        VaultError::wrap(|| unsafe { ockam_random_urandom_init(&mut context) })?;

        Ok(Self(context))
    }
}
impl AsRef<ockam_random_t> for Random {
    fn as_ref(&self) -> &ockam_random_t {
        &self.0
    }
}
impl AsMut<ockam_random_t> for Random {
    fn as_mut(&mut self) -> &mut ockam_random_t {
        &mut self.0
    }
}
impl Drop for Random {
    fn drop(&mut self) {
        use ockam_vault_sys::ockam_random_deinit;

        unsafe {
            ockam_random_deinit(&mut self.0);
        }
    }
}

/// Represents a single instance of an Ockam vault context
#[derive(Debug)]
pub struct Vault {
    random: Random,
    context: ockam_vault_t,
}
impl Vault {
    pub fn new() -> VaultResult<Self> {
        use ockam_vault_sys::ockam_vault_default_init;

        let mut random = Random::new()?;
        let context = {
            let mut attributes = ockam_vault_default_attributes_t {
                memory: RustAlloc::new().as_mut_ptr(),
                random: random.as_mut(),
                features: 0,
            };
            let mut context = ockam_vault_t {
                dispatch: ptr::null_mut(),
                context: ptr::null_mut(),
            };
            VaultError::wrap(|| unsafe {
                ockam_vault_default_init(&mut context, &mut attributes)
            })?;
            context
        };

        Ok(Self { random, context })
    }
}
impl AsMut<ockam_vault_t> for Vault {
    fn as_mut(&mut self) -> &mut ockam_vault_t {
        &mut self.context
    }
}
impl Drop for Vault {
    fn drop(&mut self) {
        use ockam_vault_sys::ockam_vault_deinit;

        unsafe {
            ockam_vault_deinit(&mut self.context);
        }
    }
}
impl Vault {
    /// Writes random bytes to the given slice
    ///
    /// Returns `Ok` if successful, `Err(reason)` otherwise
    ///
    /// It is recommended to use the `rand` module rather than use this directly
    pub fn random(&mut self, bytes: &mut [u8]) -> VaultResult<()> {
        let ptr = bytes.as_mut_ptr();
        let len = bytes.len();
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::ockam_vault_random_bytes_generate(self.as_mut(), ptr, len)
        })
    }

    /// Perform a SHA256 operation on the message passed in.
    pub fn sha256(&mut self, bytes: &[u8]) -> VaultResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(32);
        let bytes_written = self.sha256_with_buffer(bytes, buffer.as_mut_slice())?;

        unsafe {
            buffer.set_len(bytes_written);
        }

        Ok(buffer)
    }

    /// Same as `sha256`, but takes an output buffer to write to
    pub fn sha256_with_buffer(&mut self, bytes: &[u8], buffer: &mut [u8]) -> VaultResult<usize> {
        let ptr = bytes.as_ptr() as *mut _;
        let len = bytes.len();

        let buffer_ptr = buffer.as_mut_ptr();
        let buffer_len = buffer.len();
        let mut bytes_written = 0;
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::ockam_vault_sha256(
                self.as_mut(),
                ptr,
                len,
                buffer_ptr,
                buffer_len,
                &mut bytes_written,
            )
        })?;

        Ok(bytes_written)
    }

    /// Generate an ockam secret
    ///
    /// Attributes struct must specify the configuration for the type of secret to generate.
    ///
    /// For EC keys and AES keys, length is ignored.
    pub fn generate_secret(&mut self, attributes: SecretAttributes) -> VaultResult<Secret> {
        let secret = {
            let mut secret = ockam_vault_secret_t {
                attributes: unsafe {
                    mem::transmute::<SecretAttributes, ockam_vault_secret_attributes_t>(
                        attributes.clone(),
                    )
                },
                context: ptr::null_mut(),
            };

            VaultError::wrap(|| unsafe {
                ockam_vault_sys::ockam_vault_secret_generate(
                    self.as_mut(),
                    &mut secret,
                    attributes.as_ref(),
                )
            })?;

            Secret(secret)
        };

        Ok(secret)
    }

    /// Import the specified data into the supplied ockam vault secret
    pub fn import_secret(
        &mut self,
        input: &[u8],
        mut attributes: SecretAttributes,
    ) -> VaultResult<Secret> {
        attributes.length = input
            .len()
            .try_into()
            .map_err(|_| VaultError::InvalidSize)?;

        let secret = {
            let mut secret = ockam_vault_secret_t {
                attributes: unsafe {
                    mem::transmute::<SecretAttributes, ockam_vault_secret_attributes_t>(
                        attributes.clone(),
                    )
                },
                context: ptr::null_mut(),
            };
            let input_ptr = input.as_ptr();
            let input_len = input.len();

            VaultError::wrap(|| unsafe {
                ockam_vault_sys::ockam_vault_secret_import(
                    self.as_mut(),
                    &mut secret,
                    attributes.as_ref(),
                    input_ptr,
                    input_len,
                )
            })?;

            Secret(secret)
        };

        Ok(secret)
    }

    /// Export data from an ockam vault secret into a new buffer
    pub fn export_secret(&mut self, secret: &Secret) -> VaultResult<Vec<u8>> {
        let attrs = self.get_secret_attributes(secret)?;
        let mut buffer = Vec::with_capacity(attrs.length as usize);
        let bytes_written = self.export_secret_with_buffer(secret, buffer.as_mut_slice())?;

        unsafe {
            buffer.set_len(bytes_written);
        }

        Ok(buffer)
    }

    /// Export data from an ockam vault secret into the supplied output buffer
    pub fn export_secret_with_buffer(
        &mut self,
        secret: &Secret,
        buffer: &mut [u8],
    ) -> VaultResult<usize> {
        let mut bytes_written = 0;
        VaultError::wrap(|| unsafe {
            let ptr = buffer.as_mut_ptr();
            let len = buffer.len();
            ockam_vault_sys::ockam_vault_secret_export(
                self.as_mut(),
                secret.as_ref() as *const _ as *mut _,
                ptr,
                len,
                &mut bytes_written,
            )
        })?;

        Ok(bytes_written)
    }

    /// Retrive the attributes for a specified secret
    pub fn get_secret_attributes(&mut self, secret: &Secret) -> VaultResult<SecretAttributes> {
        let attrs = {
            let mut attrs = SecretAttributes::default();
            VaultError::wrap(|| unsafe {
                ockam_vault_sys::ockam_vault_secret_attributes_get(
                    self.as_mut(),
                    secret.as_ref() as *const _ as *mut _,
                    attrs.as_mut(),
                )
            })?;

            attrs
        };

        Ok(attrs)
    }

    /// Retrieve the public key from an ockam vault secret
    pub fn get_public_key(&mut self, secret: &Secret) -> VaultResult<Vec<u8>> {
        let attrs = self.get_secret_attributes(secret)?;
        let mut buffer = Vec::with_capacity(attrs.length as usize);
        let bytes_written = self.get_public_key_with_buffer(secret, buffer.as_mut_slice())?;

        unsafe {
            buffer.set_len(bytes_written);
        }

        Ok(buffer)
    }

    /// Retrieve the public key from an ockam vault secret, writing it to the provided buffer.
    ///
    /// Returns the number of bytes written to the buffer.
    pub fn get_public_key_with_buffer(
        &mut self,
        secret: &Secret,
        buffer: &mut [u8],
    ) -> VaultResult<usize> {
        let ptr = buffer.as_mut_ptr();
        let len = buffer.len();
        let mut bytes_written = 0;
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::ockam_vault_secret_publickey_get(
                self.as_mut(),
                secret.as_ref() as *const _ as *mut _,
                ptr,
                len,
                &mut bytes_written,
            )
        })?;

        Ok(bytes_written)
    }

    /// Set the type of secret.
    ///
    /// **NOTE:** EC secrets can not be changed
    pub fn set_secret_type(&mut self, secret: &mut Secret, ty: SecretType) -> VaultResult<()> {
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::ockam_vault_secret_type_set(self.as_mut(), secret.as_mut(), ty.into())
        })
    }

    /// Delete an ockam vault secret
    pub fn destroy_secret(&mut self, secret: &mut Secret) -> VaultResult<()> {
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::ockam_vault_secret_destroy(self.as_mut(), secret.as_mut())
        })
    }

    /// Perform an ECDH operation on the supplied ockam vault secret and peer public key.
    ///
    /// The result is another ockam vault secret of type unknown.
    pub fn ecdh(&mut self, private_key: &Secret, peer_pubkey: &[u8]) -> VaultResult<Secret> {
        let ptr = peer_pubkey.as_ptr() as *mut _;
        let len = peer_pubkey.len();
        let attributes = self.get_secret_attributes(private_key)?;
        let mut shared_secret = ockam_vault_secret_t {
            attributes: unsafe {
                mem::transmute::<SecretAttributes, ockam_vault_secret_attributes_t>(
                    attributes.clone(),
                )
            },
            context: ptr::null_mut(),
        };
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::ockam_vault_ecdh(
                self.as_mut(),
                private_key.as_ref() as *const _ as *mut _,
                ptr,
                len,
                &mut shared_secret,
            )
        })?;

        Ok(Secret(shared_secret))
    }

    /// Perform an HMAC-SHA256 based key derivation function on the supplied salt and input key
    /// material
    pub fn hkdf_sha256(
        &mut self,
        salt: &Secret,
        input_key_material: Option<&Secret>,
        num_derived_outputs: u8,
    ) -> VaultResult<Vec<Secret>> {
        let len = num_derived_outputs as usize;

        let mut derived_outputs = unsafe {
            let mut ds = Vec::with_capacity(len);
            ds.set_len(len);
            for i in 0..len {
                ds[i] = MaybeUninit::<ockam_vault_secret_t>::zeroed();
            }
            ds
        };

        VaultError::wrap(|| unsafe {
            ockam_vault_sys::ockam_vault_hkdf_sha256(
                self.as_mut(),
                salt.as_ref() as *const _ as *mut _,
                input_key_material
                    .map(|ikm| ikm.as_ref() as *const _ as *mut _)
                    .unwrap_or(ptr::null_mut()),
                num_derived_outputs,
                derived_outputs.as_mut_ptr() as *mut _,
            )
        })?;

        let derived = {
            unsafe {
                derived_outputs.set_len(len);
            }
            let mut derived = Vec::with_capacity(len);
            for output in derived_outputs.drain(..) {
                derived.push(Secret(unsafe { MaybeUninit::assume_init(output) }));
            }
            derived
        };

        Ok(derived)
    }

    /// Encrypt a payload using AES-GCM
    pub fn aead_aes_gcm_encrypt(
        &mut self,
        key: &Secret,
        nonce: u16,
        additional_data: Option<&[u8]>,
        plaintext: &[u8],
    ) -> VaultResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(plaintext.len() + 16);
        let bytes_written = self.aead_aes_gcm_encrypt_with_buffer(
            key,
            nonce,
            additional_data,
            plaintext,
            buffer.as_mut_slice(),
        )?;
        unsafe {
            buffer.set_len(bytes_written);
        }
        Ok(buffer)
    }

    /// Same as `aead_aes_gcm_encrypt`, but takes a buffer to write the output to
    pub fn aead_aes_gcm_encrypt_with_buffer(
        &mut self,
        key: &Secret,
        nonce: u16,
        additional_data: Option<&[u8]>,
        plaintext: &[u8],
        buffer: &mut [u8],
    ) -> VaultResult<usize> {
        let mut bytes_written = 0;
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::ockam_vault_aead_aes_gcm_encrypt(
                self.as_mut(),
                key.as_ref() as *const _ as *mut _,
                nonce,
                additional_data
                    .map(|b| b.as_ptr())
                    .unwrap_or_else(ptr::null),
                additional_data.map(|b| b.len()).unwrap_or_default(),
                plaintext.as_ptr(),
                plaintext.len(),
                buffer.as_mut_ptr(),
                buffer.len(),
                &mut bytes_written,
            )
        })?;

        Ok(bytes_written)
    }

    /// Decrypt a payload using AES-GCM
    pub fn aead_aes_gcm_decrypt(
        &mut self,
        key: &Secret,
        nonce: u16,
        additional_data: Option<&[u8]>,
        ciphertext_and_tag: &[u8],
    ) -> VaultResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(ciphertext_and_tag.len() - 16);
        let bytes_written = self.aead_aes_gcm_decrypt_with_buffer(
            key,
            nonce,
            additional_data,
            ciphertext_and_tag,
            buffer.as_mut_slice(),
        )?;
        unsafe {
            buffer.set_len(bytes_written);
        }
        Ok(buffer)
    }

    /// Same as `aead_aes_gcm_decrypt`, but takes a buffer to write the output to
    pub fn aead_aes_gcm_decrypt_with_buffer(
        &mut self,
        key: &Secret,
        nonce: u16,
        additional_data: Option<&[u8]>,
        ciphertext_and_tag: &[u8],
        buffer: &mut [u8],
    ) -> VaultResult<usize> {
        let mut bytes_written = 0;
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::ockam_vault_aead_aes_gcm_decrypt(
                self.as_mut(),
                key.as_ref() as *const _ as *mut _,
                nonce,
                additional_data
                    .map(|b| b.as_ptr())
                    .unwrap_or_else(|| ptr::null()),
                additional_data.map(|b| b.len()).unwrap_or_default(),
                ciphertext_and_tag.as_ptr(),
                ciphertext_and_tag.len(),
                buffer.as_mut_ptr(),
                buffer.len(),
                &mut bytes_written,
            )
        })?;

        Ok(bytes_written)
    }
}

impl rand_core::RngCore for Vault {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        let mut bytes = [0u8; 4];
        self.fill_bytes(&mut bytes[..]);
        u32::from_ne_bytes(bytes)
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        let mut bytes = [0u8; 8];
        self.fill_bytes(&mut bytes[..]);
        u64::from_ne_bytes(bytes)
    }

    #[inline]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.random(dest).unwrap();
    }

    #[inline]
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        Ok(self.fill_bytes(dest))
    }
}
