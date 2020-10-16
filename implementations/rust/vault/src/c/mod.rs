use crate::error::{VaultFailError, VaultFailErrorKind};
use crate::types::{
    PublicKey, SecretKey, SecretKeyAttributes, SecretKeyContext, SecretKeyType,
    SecretPersistenceType, SecretPurposeType,
};
use crate::Vault;
use c_bindings::*;
use c_rust_memory::RustAlloc;
use std::ffi::CStr;
use zeroize::Zeroize;

/// Builder
#[derive(Debug)]
pub struct Atecc608aVaultBuilder {}

impl Default for Atecc608aVaultBuilder {
    fn default() -> Self {
        Atecc608aVaultBuilder {}
    }
}

impl Atecc608aVaultBuilder {
    /// Build
    pub fn build(self) -> Result<CVault, VaultFailError> {
        let mut c_vault = ockam_vault_t {
            dispatch: std::ptr::null_mut(),
            default_context: std::ptr::null_mut(),
            impl_context: std::ptr::null_mut(),
        };

        let memory = RustAlloc::new();

        let t = ATCAIfaceCfg__bindgen_ty_1 {
            atcai2c: ATCAIfaceCfg__bindgen_ty_1__bindgen_ty_1 {
                slave_address: 0xC0,
                bus: 1,
                baud: 100000,
            },
        };

        let mut atca_cfg = ATCAIfaceCfg {
            iface_type: ATCAIfaceType::ATCA_I2C_IFACE,
            devtype: ATCADeviceType::ATECC608A,
            __bindgen_anon_1: t,
            wake_delay: 1500,
            rx_retries: 20,
            cfg_data: std::ptr::null_mut(),
        };

        let mut io_protection = ockam_vault_atecc608a_io_protection_t {
            /* IO Protection Key is used to encrypt data sent via */
            key: [
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
                0x07, /* I2C to the ATECC608A. During init the key is       */
                0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16,
                0x17, /* written into the device. In a production system    */
                0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26,
                0x27, /* the key should be locked into the device and never */
                0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
                0x37, /* transmitted via I2C.                               */
            ],
            key_size: 32,
            slot: 6,
        };

        let mut attributes = ockam_vault_atecc608a_attributes_t {
            memory: memory.as_mut_ptr(),
            mutex: std::ptr::null_mut(),
            atca_iface_cfg: &mut atca_cfg,
            io_protection: &mut io_protection,
        };

        let error = unsafe { ockam_vault_atecc608a_init(&mut c_vault, &mut attributes) };
        CVault::handle_error(error)?;

        Ok(CVault::new(c_vault))
    }
}

// TODO: Should be thread-safe?
/// Represents a single instance of an Ockam vault context
#[derive(Debug)]
pub struct CVault {
    context: ockam_vault_t,
}

impl CVault {
    fn new(vault: ockam_vault_t) -> Self {
        CVault { context: vault }
    }

    fn handle_error(error: ockam_error_t) -> Result<(), VaultFailError> {
        if ockam_error_is_none(&error) {
            Ok(())
        } else {
            Err(error.into())
        }
    }

    fn extract_handle(context: SecretKeyContext) -> Result<ockam_vault_secret_t, VaultFailError> {
        if let SecretKeyContext::CHandle { handle } = context {
            Ok(handle)
        } else {
            Err(VaultFailError::from(VaultFailErrorKind::InvalidSecret))
        }
    }

    fn nonce_to_u16(nonce: &[u8]) -> Result<u16, VaultFailError> {
        if nonce.len() != 12 {
            return Err(VaultFailError::from(VaultFailErrorKind::InvalidSecret));
        }

        Ok(u16::from_be_bytes([nonce.as_ref()[10], nonce.as_ref()[11]]))
    }
}

impl Zeroize for CVault {
    fn zeroize(&mut self) {}
}

impl Into<VaultFailError> for ockam_error_t {
    fn into(self) -> VaultFailError {
        let str = unsafe { CStr::from_ptr(self.domain) };

        VaultFailError::from_msg(
            VaultFailErrorKind::InvalidContext,
            format!("{}: {}", str.to_str().unwrap(), self.code),
        )
    }
}

impl From<ockam_vault_secret_type_t> for SecretKeyType {
    fn from(type_: ockam_vault_secret_type_t) -> Self {
        match type_ {
            ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY => Self::P256,
            ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_AES128_KEY => Self::Aes128,
            ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_AES256_KEY => Self::Aes256,
            ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY => {
                Self::Curve25519
            }
            ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_BUFFER => Self::Buffer(0), // FIXME
        }
    }
}

impl Into<ockam_vault_secret_type_t> for SecretKeyType {
    fn into(self) -> ockam_vault_secret_type_t {
        match self {
            Self::P256 => ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY,
            Self::Aes128 => ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_AES128_KEY,
            Self::Aes256 => ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_AES256_KEY,
            Self::Curve25519 => {
                ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY
            }
            Self::Buffer(_) => ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_BUFFER, // FIXME
        }
    }
}

impl From<ockam_vault_secret_purpose_t> for SecretPurposeType {
    fn from(purpose: ockam_vault_secret_purpose_t) -> Self {
        match purpose {
            ockam_vault_secret_purpose_t::OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT => {
                Self::KeyAgreement
            }
        }
    }
}

impl Into<ockam_vault_secret_purpose_t> for SecretPurposeType {
    fn into(self) -> ockam_vault_secret_purpose_t {
        match self {
            Self::KeyAgreement => {
                ockam_vault_secret_purpose_t::OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT
            }
        }
    }
}

impl From<ockam_vault_secret_persistence_t> for SecretPersistenceType {
    fn from(persistence: ockam_vault_secret_persistence_t) -> Self {
        match persistence {
            ockam_vault_secret_persistence_t::OCKAM_VAULT_SECRET_EPHEMERAL => Self::Ephemeral,
            ockam_vault_secret_persistence_t::OCKAM_VAULT_SECRET_PERSISTENT => Self::Persistent,
        }
    }
}

impl Into<ockam_vault_secret_persistence_t> for SecretPersistenceType {
    fn into(self) -> ockam_vault_secret_persistence_t {
        match self {
            Self::Ephemeral => ockam_vault_secret_persistence_t::OCKAM_VAULT_SECRET_EPHEMERAL,
            Self::Persistent => ockam_vault_secret_persistence_t::OCKAM_VAULT_SECRET_PERSISTENT,
        }
    }
}

impl From<ockam_vault_secret_attributes_t> for SecretKeyAttributes {
    fn from(attrs: ockam_vault_secret_attributes_t) -> Self {
        SecretKeyAttributes {
            xtype: attrs.type_.into(),
            persistence: attrs.persistence.into(),
            purpose: attrs.purpose.into(),
        }
    }
}

impl Into<ockam_vault_secret_attributes_t> for SecretKeyAttributes {
    fn into(self) -> ockam_vault_secret_attributes_t {
        ockam_vault_secret_attributes_t {
            length: 0, //FIXME
            type_: self.xtype.into(),
            purpose: self.purpose.into(),
            persistence: self.persistence.into(),
        }
    }
}

impl Vault for CVault {
    fn random(&mut self, data: &mut [u8]) -> Result<(), VaultFailError> {
        let ptr = data.as_mut_ptr();
        let len = data.len();
        let error = unsafe { ockam_vault_random_bytes_generate(&mut self.context, ptr, len) };
        Self::handle_error(error)?;

        Ok(())
    }

    fn sha256<B: AsRef<[u8]>>(&self, data: B) -> Result<[u8; 32], VaultFailError> {
        const SHA256_DIGEST_SIZE: usize = 32;
        let mut buffer = [0u8; SHA256_DIGEST_SIZE];
        let mut output_size: usize = 0;
        let output_ptr = buffer.as_mut_ptr();
        let input_ptr = data.as_ref().as_ptr();
        let input_len = data.as_ref().len();

        let error = unsafe {
            ockam_vault_sha256(
                &self.context,
                input_ptr,
                input_len,
                output_ptr,
                SHA256_DIGEST_SIZE,
                &mut output_size,
            )
        };
        Self::handle_error(error)?;

        if output_size != SHA256_DIGEST_SIZE {
            return Err(VaultFailError::from(VaultFailErrorKind::InvalidSecret));
        }

        Ok(buffer)
    }

    fn secret_generate(
        &mut self,
        attributes: SecretKeyAttributes,
    ) -> Result<SecretKeyContext, VaultFailError> {
        let mut handle = ockam_vault_secret_t::default();

        let error = unsafe {
            ockam_vault_secret_generate(&mut self.context, &mut handle, &attributes.into())
        };
        Self::handle_error(error)?;

        Ok(SecretKeyContext::CHandle { handle })
    }

    fn secret_import(
        &mut self,
        secret: &SecretKey,
        attributes: SecretKeyAttributes,
    ) -> Result<SecretKeyContext, VaultFailError> {
        let slice = match secret {
            SecretKey::P256(arr) => &arr[..],
            SecretKey::Curve25519(arr) => &arr[..],
            SecretKey::Buffer(vec) => &vec[..],
            SecretKey::Aes128(arr) => &arr[..],
            SecretKey::Aes256(arr) => &arr[..],
        };

        let ptr = slice.as_ptr();
        let len = slice.len();

        let mut handle = ockam_vault_secret_t::default();

        let error = unsafe {
            ockam_vault_secret_import(&mut self.context, &mut handle, &attributes.into(), ptr, len)
        };
        Self::handle_error(error)?;

        Ok(SecretKeyContext::CHandle { handle })
    }

    fn secret_export(&mut self, context: SecretKeyContext) -> Result<SecretKey, VaultFailError> {
        let mut handle = Self::extract_handle(context)?;

        const MAX_SECRET_SIZE: usize = 65;

        let mut buffer = [0u8; MAX_SECRET_SIZE];
        let output_ptr = buffer.as_mut_ptr();
        let mut output_size: usize = 0;

        let error = unsafe {
            ockam_vault_secret_export(
                &mut self.context,
                &mut handle,
                output_ptr,
                MAX_SECRET_SIZE,
                &mut output_size,
            )
        };
        Self::handle_error(error)?;

        match handle.attributes.type_ {
            ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY => {
                if output_size != 32 {
                    return Err(VaultFailError::from(VaultFailErrorKind::InvalidSecret));
                }
                Ok(SecretKey::P256(*array_ref![buffer, 0, 32]))
            }
            ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY => {
                if output_size != 32 {
                    return Err(VaultFailError::from(VaultFailErrorKind::InvalidSecret));
                }
                Ok(SecretKey::Curve25519(*array_ref![buffer, 0, 32]))
            }
            ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_AES256_KEY => {
                if output_size != 32 {
                    return Err(VaultFailError::from(VaultFailErrorKind::InvalidSecret));
                }
                Ok(SecretKey::Aes256(*array_ref![buffer, 0, 32]))
            }
            ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_AES128_KEY => {
                if output_size != 16 {
                    return Err(VaultFailError::from(VaultFailErrorKind::InvalidSecret));
                }
                Ok(SecretKey::Aes128(*array_ref![buffer, 0, 16]))
            }
            ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_BUFFER => {
                let mut res = buffer.to_vec();
                res.resize(output_size, 0);
                Ok(SecretKey::Buffer(res))
            }
        }
    }

    fn secret_attributes_get(
        &mut self,
        context: SecretKeyContext,
    ) -> Result<SecretKeyAttributes, VaultFailError> {
        let mut handle = Self::extract_handle(context)?;
        let mut attributes = ockam_vault_secret_attributes_t::default();

        let error = unsafe {
            ockam_vault_secret_attributes_get(&mut self.context, &mut handle, &mut attributes)
        };
        Self::handle_error(error)?;

        Ok(attributes.into())
    }

    fn secret_public_key_get(
        &mut self,
        context: SecretKeyContext,
    ) -> Result<PublicKey, VaultFailError> {
        let mut handle = Self::extract_handle(context)?;

        match handle.attributes.type_ {
            ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY
            | ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY => {}
            _ => return Err(VaultFailError::from(VaultFailErrorKind::InvalidSecret)),
        };

        const MAX_PUBLIC_KEY_SIZE: usize = 65;

        let mut buffer = [0u8; MAX_PUBLIC_KEY_SIZE];
        let output_ptr = buffer.as_mut_ptr();
        let mut output_size: usize = 0;
        let error = unsafe {
            ockam_vault_secret_publickey_get(
                &mut self.context,
                &mut handle,
                output_ptr,
                MAX_PUBLIC_KEY_SIZE,
                &mut output_size,
            )
        };
        Self::handle_error(error)?;

        match handle.attributes.type_ {
            ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY => {
                if output_size != 65 {
                    return Err(VaultFailError::from(VaultFailErrorKind::InvalidSecret));
                }
                Ok(PublicKey::P256(*array_ref![buffer, 0, 65]))
            }
            ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY => {
                if output_size != 32 {
                    return Err(VaultFailError::from(VaultFailErrorKind::InvalidSecret));
                }
                Ok(PublicKey::Curve25519(*array_ref![buffer, 0, 32]))
            }
            _ => Err(VaultFailError::from(VaultFailErrorKind::InvalidSecret)),
        }
    }

    fn secret_destroy(&mut self, context: SecretKeyContext) -> Result<(), VaultFailError> {
        let mut handle = Self::extract_handle(context)?;

        let error = unsafe { ockam_vault_secret_destroy(&mut self.context, &mut handle) };
        Self::handle_error(error)?;

        Ok(())
    }

    fn ec_diffie_hellman(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
    ) -> Result<SecretKeyContext, VaultFailError> {
        let mut handle = Self::extract_handle(context)?;

        let slice = match &peer_public_key {
            PublicKey::P256(arr) => &arr[..],
            PublicKey::Curve25519(arr) => &arr[..],
        };

        let ptr = slice.as_ptr();
        let len = slice.len();

        let mut secret = ockam_vault_secret_t::default();

        let error =
            unsafe { ockam_vault_ecdh(&mut self.context, &mut handle, ptr, len, &mut secret) };
        Self::handle_error(error)?;

        Ok(SecretKeyContext::CHandle { handle })
    }

    fn ec_diffie_hellman_hkdf_sha256(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
        salt: SecretKeyContext,
        info: &[u8],
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<SecretKeyContext>, VaultFailError> {
        unimplemented!()
    }

    fn hkdf_sha256(
        &mut self,
        salt: SecretKeyContext,
        info: &[u8],
        ikm: Option<SecretKeyContext>,
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<SecretKeyContext>, VaultFailError> {
        // FIXME: info not supported
        // FIXME: only one output supported
        let mut salt = Self::extract_handle(salt)?;

        let ikm: *mut ockam_vault_secret_t = match ikm {
            Some(ikm) => &mut Self::extract_handle(ikm)?,
            None => std::ptr::null_mut(),
        };

        let mut res = Vec::<ockam_vault_secret_t>::new();
        res.resize(output_attributes.len(), ockam_vault_secret_t::default());

        let error = unsafe {
            ockam_vault_hkdf_sha256(
                &mut self.context,
                &mut salt,
                ikm,
                output_attributes.len() as u8,
                res.as_mut_ptr(),
            )
        };
        Self::handle_error(error)?;

        let res = res
            .iter()
            .map(|&handle| SecretKeyContext::CHandle { handle })
            .collect();

        Ok(res)
    }

    fn aead_aes_gcm_encrypt<B: AsRef<[u8]>, C: AsRef<[u8]>, D: AsRef<[u8]>>(
        &mut self,
        context: SecretKeyContext,
        plaintext: B,
        nonce: C,
        aad: D,
    ) -> Result<Vec<u8>, VaultFailError> {
        let mut handle = Self::extract_handle(context)?;

        let aad_ptr = aad.as_ref().as_ptr();
        let aad_len = aad.as_ref().len();

        let plaintext_ptr = plaintext.as_ref().as_ptr();
        let plaintext_len = plaintext.as_ref().len();

        // FIXME: Move computation to implementation side
        let cipher_text_len = plaintext_len + 16;

        let mut ciphertext = Vec::<u8>::with_capacity(cipher_text_len);

        let ciphertext_ptr = ciphertext.as_mut_ptr();
        let mut output_size = 0;

        let nonce = Self::nonce_to_u16(nonce.as_ref())?;

        let error = unsafe {
            ockam_vault_aead_aes_gcm_encrypt(
                &mut self.context,
                &mut handle,
                nonce,
                aad_ptr,
                aad_len,
                plaintext_ptr,
                plaintext_len,
                ciphertext_ptr,
                cipher_text_len,
                &mut output_size,
            )
        };
        Self::handle_error(error)?;

        if output_size != cipher_text_len {
            return Err(VaultFailError::from(VaultFailErrorKind::InvalidSecret));
        }

        Ok(ciphertext)
    }

    fn aead_aes_gcm_decrypt<B: AsRef<[u8]>, C: AsRef<[u8]>, D: AsRef<[u8]>>(
        &mut self,
        context: SecretKeyContext,
        cipher_text: B,
        nonce: C,
        aad: D,
    ) -> Result<Vec<u8>, VaultFailError> {
        let mut handle = Self::extract_handle(context)?;

        let aad_ptr = aad.as_ref().as_ptr();
        let aad_len = aad.as_ref().len();

        let ciphertext_ptr = cipher_text.as_ref().as_ptr();
        let ciphertext_len = cipher_text.as_ref().len();

        if ciphertext_len < 16 {
            return Err(VaultFailError::from(VaultFailErrorKind::InvalidSecret));
        }

        // FIXME: Move computation to implementation side
        let plaintext_len = ciphertext_len - 16;

        let mut plaintext = Vec::<u8>::with_capacity(plaintext_len);

        let plaintext_ptr = plaintext.as_mut_ptr();
        let mut output_size = 0;

        let nonce = Self::nonce_to_u16(nonce.as_ref())?;

        let error = unsafe {
            ockam_vault_aead_aes_gcm_decrypt(
                &mut self.context,
                &mut handle,
                nonce,
                aad_ptr,
                aad_len,
                ciphertext_ptr,
                ciphertext_len,
                plaintext_ptr,
                plaintext_len,
                &mut output_size,
            )
        };
        Self::handle_error(error)?;

        if output_size != plaintext_len {
            return Err(VaultFailError::from(VaultFailErrorKind::InvalidSecret));
        }

        Ok(plaintext)
    }

    fn deinit(&mut self) {
        let error = unsafe { ockam_vault_deinit(&mut self.context) };
        Self::handle_error(error).unwrap(); // FIXME
    }

    fn sign<B: AsRef<[u8]>>(
        &mut self,
        secret_key: SecretKeyContext,
        data: B,
    ) -> Result<[u8; 64], VaultFailError> {
        unimplemented!()
    }

    fn verify<B: AsRef<[u8]>>(
        &mut self,
        signature: [u8; 64],
        public_key: PublicKey,
        data: B,
    ) -> Result<(), VaultFailError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn new_vault() {
        let builder = Atecc608aVaultBuilder::default();
        let mut vault = builder.build().unwrap();

        let secret1 = vault
            .secret_generate(SecretKeyAttributes {
                xtype: SecretKeyType::P256,
                persistence: SecretPersistenceType::Ephemeral,
                purpose: SecretPurposeType::KeyAgreement,
            })
            .unwrap();
        let public_key1 = vault.secret_public_key_get(secret1).unwrap();

        let secret2 = vault
            .secret_generate(SecretKeyAttributes {
                xtype: SecretKeyType::P256,
                persistence: SecretPersistenceType::Ephemeral,
                purpose: SecretPurposeType::KeyAgreement,
            })
            .unwrap();
        let public_key2 = vault.secret_public_key_get(secret2).unwrap();

        let dh = vault.ec_diffie_hellman(secret1, public_key2).unwrap();

        let mut salt = [0u8; 32];
        vault.random(&mut salt).unwrap();

        let salt = vault.sha256(salt).unwrap();

        let salt_secret = vault
            .secret_import(
                &SecretKey::Buffer(salt.to_vec()),
                SecretKeyAttributes {
                    xtype: SecretKeyType::Buffer(32),
                    persistence: SecretPersistenceType::Ephemeral,
                    purpose: SecretPurposeType::KeyAgreement,
                },
            )
            .unwrap();

        let shared_secret = vault
            .hkdf_sha256(
                salt_secret,
                &[],
                Some(dh),
                vec![SecretKeyAttributes {
                    xtype: SecretKeyType::Buffer(16),
                    persistence: SecretPersistenceType::Ephemeral,
                    purpose: SecretPurposeType::KeyAgreement,
                }],
            )
            .unwrap();

        assert_eq!(shared_secret.len(), 1);
        let shared_secret = shared_secret[0];

        let mut text = [0u8; 32];
        vault.random(&mut text).unwrap();

        let mut nonce = [0u8; 12];
        nonce[10] = 2;
        nonce[11] = 3;

        let ciphertext = vault
            .aead_aes_gcm_encrypt(shared_secret, &text, &nonce, &[])
            .unwrap();

        // TODO:
        // vault.sha256()
        // vault.random()
        // vault.secret_import()
        // vault.secret_attributes_get()
        // vault.secret_destroy()
        // vault.sign()
        // vault.verify()
        // vault.ec_diffie_hellman_hkdf_sha256()
    }
}
