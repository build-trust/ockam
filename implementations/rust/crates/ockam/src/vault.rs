use std::alloc::Layout;
use std::convert::AsMut;
use std::mem::{self, MaybeUninit};
use std::ptr;

use cfg_if::cfg_if;
use thiserror::Error;

use ockam_vault_sys::ctypes::c_void;
pub use ockam_vault_sys::OckamFeatures as VaultFeatures;
use ockam_vault_sys::{OckamError, OckamMemory};
use ockam_vault_sys::{OckamVaultCtx, OckamVaultDefaultConfig};

cfg_if! {
    if #[cfg(feature = "term_encoding")] {
        use rustler;
        use rustler::NifUnitEnum;
    }
}

pub type VaultResult<T> = Result<T, ()>;

#[derive(Error, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum VaultError {
    #[error("ok")]
    Ok = 0,
    #[error("vault_error")]
    Failed,
}
impl VaultError {
    #[inline]
    fn wrap<F>(mut fun: F) -> VaultResult<()>
    where
        F: FnMut() -> ockam_vault_sys::OckamError,
    {
        match fun().into() {
            Self::Ok => Ok(()),
            Self::Failed => Err(()),
        }
    }
}
impl From<OckamError> for VaultError {
    fn from(err: OckamError) -> Self {
        if err == ockam_vault_sys::kOckamErrorNone {
            Self::Ok
        } else {
            Self::Failed
        }
    }
}
impl Into<OckamError> for VaultError {
    fn into(self) -> OckamError {
        match self {
            Self::Ok => 0,
            Self::Failed => 1,
        }
    }
}

#[cfg(feature = "term_encoding")]
rustler::atoms! {
    ok,
    vault_error,
}

#[cfg(feature = "term_encoding")]
impl rustler::Encoder for VaultError {
    fn encode<'c>(&self, env: rustler::Env<'c>) -> rustler::Term<'c> {
        use rustler::Atom;
        let string = self.to_string();
        let atom = Atom::try_from_bytes(env, string.as_bytes())
            .unwrap()
            .unwrap();
        atom.to_term(env)
    }
}

/// Support key types in Ockam Vault
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "term_encoding", derive(NifUnitEnum))]
pub enum KeyType {
    Static = 0,
    Ephemeral,
}
impl From<ockam_vault_sys::OckamVaultKey> for KeyType {
    fn from(value: ockam_vault_sys::OckamVaultKey) -> Self {
        use ockam_vault_sys::OckamVaultKey::*;
        match value {
            kOckamVaultKeyStatic => Self::Static,
            kOckamVaultKeyEphemeral => Self::Ephemeral,
            _ => unreachable!(),
        }
    }
}
impl Into<ockam_vault_sys::OckamVaultKey> for KeyType {
    fn into(self) -> ockam_vault_sys::OckamVaultKey {
        use ockam_vault_sys::OckamVaultKey::*;
        match self {
            Self::Static => kOckamVaultKeyStatic,
            Self::Ephemeral => kOckamVaultKeyEphemeral,
        }
    }
}

/// Specifies the mode of operation for AES GCM
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum AesGcmMode {
    Encrypt = 0,
    Decrypt,
}

/// The elliptic curve vault will support
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "term_encoding", derive(NifUnitEnum))]
pub enum Curve {
    /// NIST P-256/SECP256R1
    P256 = 0,
    Curve25519,
}
impl From<ockam_vault_sys::OckamVaultEc> for Curve {
    fn from(value: ockam_vault_sys::OckamVaultEc) -> Self {
        use ockam_vault_sys::OckamVaultEc::*;
        match value {
            kOckamVaultEcP256 => Self::P256,
            kOckamVaultEcCurve25519 => Self::Curve25519,
            _ => unreachable!(),
        }
    }
}
impl Into<ockam_vault_sys::OckamVaultEc> for Curve {
    fn into(self) -> ockam_vault_sys::OckamVaultEc {
        use ockam_vault_sys::OckamVaultEc::*;
        match self {
            Self::P256 => kOckamVaultEcP256,
            Self::Curve25519 => kOckamVaultEcCurve25519,
        }
    }
}

#[repr(C)]
pub struct VaultAlloc {
    inner: OckamMemory,
}
impl AsMut<OckamMemory> for VaultAlloc {
    fn as_mut(&mut self) -> &mut OckamMemory {
        &mut self.inner
    }
}
impl VaultAlloc {
    pub fn new() -> Self {
        Self {
            inner: OckamMemory {
                Create: None,
                Alloc: Some(VaultAlloc::alloc_impl),
                Free: Some(VaultAlloc::free_impl),
                Copy: Some(VaultAlloc::memcopy_impl),
                Set: Some(VaultAlloc::memset_impl),
                Move: Some(VaultAlloc::memmove_impl),
            },
        }
    }

    unsafe extern "C" fn alloc_impl(buffer: *mut *mut c_void, size: usize) -> OckamError {
        let layout_result = Layout::from_size_align(size, mem::align_of::<c_void>());
        if let Ok(layout) = layout_result {
            let ptr = std::alloc::alloc(layout);
            if !ptr.is_null() {
                buffer.write(ptr as *mut _);
                return VaultError::Ok.into();
            }
        }

        VaultError::Failed.into()
    }

    unsafe extern "C" fn free_impl(ptr: *mut c_void, size: usize) -> OckamError {
        let layout_result = Layout::from_size_align(size, mem::align_of::<c_void>());
        if let Ok(layout) = layout_result {
            std::alloc::dealloc(ptr as *mut _, layout);
            VaultError::Ok
        } else {
            VaultError::Failed
        }
        .into()
    }

    unsafe extern "C" fn memcopy_impl(
        dst: *mut c_void,
        src: *mut c_void,
        size: usize,
    ) -> OckamError {
        core::intrinsics::copy_nonoverlapping(src as *mut u8, dst as *mut u8, size);
        VaultError::Ok.into()
    }

    unsafe extern "C" fn memset_impl(ptr: *mut c_void, byte: u8, count: usize) -> OckamError {
        core::intrinsics::write_bytes(ptr as *mut u8, byte, count);
        VaultError::Ok.into()
    }

    unsafe extern "C" fn memmove_impl(
        dst: *mut c_void,
        src: *mut c_void,
        size: usize,
    ) -> OckamError {
        core::intrinsics::copy(src as *mut u8, dst as *mut u8, size);
        VaultError::Ok.into()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SoftwareVault {
    features: VaultFeatures,
    curve: Curve,
}
impl Default for SoftwareVault {
    fn default() -> Self {
        Self {
            features: VaultFeatures::OCKAM_VAULT_FEATURE_ALL,
            curve: Curve::Curve25519,
        }
    }
}
impl Into<OckamVaultDefaultConfig> for SoftwareVault {
    fn into(self) -> OckamVaultDefaultConfig {
        OckamVaultDefaultConfig {
            features: self.features.0,
            ec: self.curve.into(),
        }
    }
}
impl AsMut<OckamVaultDefaultConfig> for SoftwareVault {
    fn as_mut(&mut self) -> &mut OckamVaultDefaultConfig {
        unsafe { mem::transmute::<&mut Self, &mut OckamVaultDefaultConfig>(self) }
    }
}

pub struct VaultContext {
    alloc: VaultAlloc,
    features: VaultFeatures,
    config: SoftwareVault,
    ec: Curve,
    inner: *mut OckamVaultCtx,
}
impl VaultContext {
    fn new(features: VaultFeatures, ec: Curve) -> VaultResult<Self> {
        let mut context = MaybeUninit::<*mut OckamVaultCtx>::uninit();
        let mut config = SoftwareVault::default();
        let mut alloc = VaultAlloc::new();

        let result: VaultError = unsafe {
            ockam_vault_sys::VaultDefaultCreate(
                context.as_mut_ptr(),
                config.as_mut(),
                alloc.as_mut(),
            )
            .into()
        };
        if let VaultError::Ok = result {
            Ok(Self {
                alloc,
                features,
                config,
                ec,
                inner: unsafe { MaybeUninit::assume_init(context) },
            })
        } else {
            Err(())
        }
    }
}
impl AsMut<OckamVaultCtx> for VaultContext {
    fn as_mut(&mut self) -> &mut OckamVaultCtx {
        unsafe { &mut *self.inner }
    }
}
impl Drop for VaultContext {
    fn drop(&mut self) {
        unsafe {
            ockam_vault_sys::VaultDefaultDestroy(self.inner);
        }
    }
}

pub struct Vault {
    context: VaultContext,
}
impl Vault {
    pub fn new(features: VaultFeatures, ec: Curve) -> VaultResult<Self> {
        let context = VaultContext::new(features, ec)?;
        Ok(Self { context })
    }

    /// Writes random bytes to the given slice
    ///
    /// Returns `Ok` if successful, `Err(reason)` otherwise
    ///
    /// It is recommended to use the `rand` module rather than use this directly
    pub fn random(&mut self, bytes: &mut [u8]) -> VaultResult<()> {
        let ptr = bytes.as_mut_ptr();
        let len = bytes.len();
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::VaultDefaultRandom(self.context.as_mut(), ptr, len)
        })
    }

    /// Generate an ECC keypair
    pub fn key_gen(&mut self, key_type: KeyType) -> VaultResult<()> {
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::VaultDefaultKeyGenerate(self.context.as_mut(), key_type.into())
        })
    }

    /// Get a public key from the vault for the given type
    pub fn get_public_key(&mut self, key_type: KeyType) -> VaultResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(32);
        self.get_public_key_with_buffer(key_type, buffer.as_mut_slice())?;
        Ok(buffer)
    }

    /// Get a public key from the vault for the given type, using the provided buffer
    pub fn get_public_key_with_buffer(
        &mut self,
        key_type: KeyType,
        buffer: &mut [u8],
    ) -> VaultResult<()> {
        let ptr = buffer.as_mut_ptr();
        let len = buffer.len();
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::VaultDefaultKeyGetPublic(
                self.context.as_mut(),
                key_type.into(),
                ptr,
                len,
            )
        })
    }

    /// Write a private key to the Ockam Vault. Should typically be used for testing only.
    pub fn write_private_key(&mut self, key_type: KeyType, privkey: &[u8]) -> VaultResult<()> {
        let ptr = privkey.as_ptr() as *mut _;
        let len = privkey.len();
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::VaultDefaultKeySetPrivate(
                self.context.as_mut(),
                key_type.into(),
                ptr,
                len,
            )
        })
    }

    /// Perform ECDH using the specified key
    ///
    /// Returns the pre-master secret key
    ///
    /// - `key_type`: The key type to use
    /// - `pubkey`: The public key to use
    pub fn ecdh(&mut self, key_type: KeyType, pubkey: &[u8]) -> VaultResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(32);
        self.ecdh_with_buffer(key_type, pubkey, buffer.as_mut_slice())?;
        Ok(buffer)
    }

    /// Same as `ecdh`, but takes an output buffer to write to
    pub fn ecdh_with_buffer(
        &mut self,
        key_type: KeyType,
        pubkey: &[u8],
        buffer: &mut [u8],
    ) -> VaultResult<()> {
        let ptr = pubkey.as_ptr() as *mut _;
        let len = pubkey.len();
        let pmk_ptr = buffer.as_mut_ptr();
        let pmk_len = buffer.len();
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::VaultDefaultEcdh(
                self.context.as_mut(),
                key_type.into(),
                ptr,
                len,
                pmk_ptr,
                pmk_len,
            )
        })
    }

    /// Perform a SHA256 operation on the message passed in.
    pub fn sha256(&mut self, bytes: &[u8]) -> VaultResult<Vec<u8>> {
        let ptr = bytes.as_ptr() as *mut _;
        let len = bytes.len();

        let mut hash = Vec::with_capacity(32);
        let hash_ptr = hash.as_mut_ptr();
        let hash_len = hash.len();
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::VaultDefaultSha256(self.context.as_mut(), ptr, len, hash_ptr, hash_len)
        })?;
        Ok(hash)
    }

    /// Perform HKDF operation on the input key material and optional salt and info.
    pub fn hkdf(&mut self, salt: &[u8], key: &[u8], info: Option<&[u8]>) -> VaultResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(32);
        self.hkdf_with_buffer(salt, key, info, buffer.as_mut_slice())?;
        Ok(buffer)
    }

    /// Same as `hkdf`, but takes an output buffer to write to
    pub fn hkdf_with_buffer(
        &mut self,
        salt: &[u8],
        key: &[u8],
        info: Option<&[u8]>,
        buffer: &mut [u8],
    ) -> VaultResult<()> {
        let result_ptr = buffer.as_mut_ptr();
        let result_len = buffer.len();
        let salt_ptr = salt.as_ptr();
        let salt_len = salt.len();
        let key_ptr = key.as_ptr();
        let key_len = key.len();
        let info_ptr = info.map(|b| b.as_ptr()).unwrap_or_else(|| ptr::null());
        let info_len = info.map(|b| b.len()).unwrap_or_default();
        VaultError::wrap(|| unsafe {
            ockam_vault_sys::VaultDefaultHkdf(
                self.context.as_mut(),
                salt_ptr as *mut _,
                salt_len,
                key_ptr as *mut _,
                key_len,
                info_ptr as *mut _,
                info_len,
                result_ptr,
                result_len,
            )
        })
    }

    /// AES GCM function for encrypt. Depending on underlying implementation, Vault may support
    /// 128, 192 and/or 256 variants.
    pub fn aes_gcm_encrypt(
        &mut self,
        input: &[u8],
        key: &[u8],
        iv: &[u8],
        additional_data: Option<&[u8]>,
        tag: &[u8],
    ) -> VaultResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(input.len());
        self.aes_gcm_with_buffer(
            AesGcmMode::Encrypt,
            input,
            key,
            iv,
            additional_data,
            tag,
            buffer.as_mut_slice(),
        )?;
        Ok(buffer)
    }

    /// Same as `aes_gcm_encrypt`, but takes a buffer to write the output to
    #[inline]
    pub fn aes_gcm_encrypt_with_buffer(
        &mut self,
        input: &[u8],
        key: &[u8],
        iv: &[u8],
        additional_data: Option<&[u8]>,
        tag: &[u8],
        buffer: &mut [u8],
    ) -> VaultResult<()> {
        self.aes_gcm_with_buffer(
            AesGcmMode::Encrypt,
            input,
            key,
            iv,
            additional_data,
            tag,
            buffer,
        )
    }

    /// AES GCM function for decrypt. Depending on underlying implementation, Vault may support
    /// 128, 192 and/or 256 variants.
    pub fn aes_gcm_decrypt(
        &mut self,
        input: &[u8],
        key: &[u8],
        iv: &[u8],
        additional_data: Option<&[u8]>,
        tag: &[u8],
    ) -> VaultResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(input.len());
        self.aes_gcm_with_buffer(
            AesGcmMode::Decrypt,
            input,
            key,
            iv,
            additional_data,
            tag,
            buffer.as_mut_slice(),
        )?;
        Ok(buffer)
    }

    /// Same as `aes_gcm_decrypt`, but takes a buffer to write the output to
    #[inline]
    pub fn aes_gcm_decrypt_with_buffer(
        &mut self,
        input: &[u8],
        key: &[u8],
        iv: &[u8],
        additional_data: Option<&[u8]>,
        tag: &[u8],
        buffer: &mut [u8],
    ) -> VaultResult<()> {
        self.aes_gcm_with_buffer(
            AesGcmMode::Decrypt,
            input,
            key,
            iv,
            additional_data,
            tag,
            buffer,
        )
    }

    fn aes_gcm_with_buffer(
        &mut self,
        mode: AesGcmMode,
        input: &[u8],
        key: &[u8],
        iv: &[u8],
        additional_data: Option<&[u8]>,
        tag: &[u8],
        buffer: &mut [u8],
    ) -> VaultResult<()> {
        let input_ptr = input.as_ptr();
        let input_len = input.len();
        let key_ptr = key.as_ptr();
        let key_len = key.len();
        let iv_ptr = iv.as_ptr();
        let iv_len = iv.len();
        let data_ptr = additional_data
            .map(|b| b.as_ptr())
            .unwrap_or_else(|| ptr::null());
        let data_len = additional_data.map(|b| b.len()).unwrap_or_default();
        let tag_ptr = tag.as_ptr();
        let tag_len = tag.len();
        let output_ptr = buffer.as_mut_ptr();
        let output_len = buffer.len();
        VaultError::wrap(|| unsafe {
            match mode {
                AesGcmMode::Encrypt => ockam_vault_sys::VaultDefaultAesGcmEncrypt(
                    self.context.as_mut(),
                    key_ptr as *mut _,
                    key_len,
                    iv_ptr as *mut _,
                    iv_len,
                    data_ptr as *mut _,
                    data_len,
                    tag_ptr as *mut _,
                    tag_len,
                    input_ptr as *mut _,
                    input_len,
                    output_ptr,
                    output_len,
                ),
                AesGcmMode::Decrypt => ockam_vault_sys::VaultDefaultAesGcmDecrypt(
                    self.context.as_mut(),
                    key_ptr as *mut _,
                    key_len,
                    iv_ptr as *mut _,
                    iv_len,
                    data_ptr as *mut _,
                    data_len,
                    tag_ptr as *mut _,
                    tag_len,
                    input_ptr as *mut _,
                    input_len,
                    output_ptr,
                    output_len,
                ),
            }
        })
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
