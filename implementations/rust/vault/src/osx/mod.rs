use crate::types::{OsKeyRing, OsxContext, SecretPurposeType};
use crate::{
    error::{VaultFailError, VaultFailErrorKind},
    software::DefaultVault,
    types::{
        PublicKey, SecretKey, SecretKeyAttributes, SecretKeyContext, SecretKeyType,
        SecretPersistenceType,
    },
    Vault,
};
use keychain_services as enclave;
use p256::arithmetic::{ProjectivePoint, AffinePoint, Scalar};
use rand::prelude::*;
use security_framework::os::macos::keychain;
use sha2::Sha256;
use std::convert::TryFrom;
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

const OCKAM_SERVICE_NAME: &str = "OckamOsxVault";

/// A Vault that interacts with the Keychain
/// and Secure Enclave Processor
pub struct OsxVault {
    ephemeral_vault: DefaultVault,
    keychain: keychain::SecKeychain,
}

impl OsxVault {
    fn unlock(&mut self) -> Result<(), VaultFailError> {
        self.keychain.unlock(None)?;
        Ok(())
    }
}

impl std::fmt::Debug for OsxVault {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "OsxVault {{ ephemeral_vault: {:?},  keychain: SecKeychain  }}",
            self.ephemeral_vault
        )
    }
}

impl Default for OsxVault {
    fn default() -> Self {
        Self {
            ephemeral_vault: DefaultVault::default(),
            keychain: keychain::SecKeychain::default().unwrap(),
        }
    }
}

impl Zeroize for OsxVault {
    fn zeroize(&mut self) {
        self.ephemeral_vault.zeroize();
    }
}

zdrop_impl!(OsxVault);

impl Vault for OsxVault {
    fn random(&mut self, data: &mut [u8]) -> Result<(), VaultFailError> {
        self.ephemeral_vault.random(data)
    }

    fn sha256<B: AsRef<[u8]>>(&self, data: B) -> Result<[u8; 32], VaultFailError> {
        self.ephemeral_vault.sha256(data)
    }

    fn secret_generate(
        &mut self,
        attributes: SecretKeyAttributes,
    ) -> Result<SecretKeyContext, VaultFailError> {
        let mut swkey_insert = |atts: SecretKeyAttributes,
                                buffer: &[u8]|
         -> Result<SecretKeyContext, VaultFailError> {
            let mut r = rand::rngs::OsRng {};
            let id = r.gen::<usize>();
            let mut bytes = atts.to_bytes().to_vec();
            bytes.extend_from_slice(buffer);
            self.unlock()?;
            self.keychain.set_generic_password(
                OCKAM_SERVICE_NAME,
                id.to_string().as_str(),
                bytes.as_slice(),
            )?;

            Ok(SecretKeyContext::KeyRing {
                id,
                os_type: OsKeyRing::Osx(OsxContext::Keychain),
            })
        };
        let mut rng = rand::rngs::OsRng {};
        match attributes.persistence {
            SecretPersistenceType::Ephemeral => {
                self.ephemeral_vault.secret_generate(attributes).map(|c| {
                    if let SecretKeyContext::Memory(id) = c {
                        SecretKeyContext::KeyRing {
                            id,
                            os_type: OsKeyRing::Osx(OsxContext::Memory),
                        }
                    } else {
                        c
                    }
                })
            }
            SecretPersistenceType::Persistent => match attributes.xtype {
                SecretKeyType::Curve25519 => {
                    let sk = x25519_dalek::StaticSecret::new(&mut rng);
                    swkey_insert(attributes, sk.to_bytes().as_ref())
                }
                SecretKeyType::P256 => {
                    // SEP doesn't support ECDH directly
                    // only for ECIES, for now just use the keychain
                    let key = p256::SecretKey::generate();
                    swkey_insert(attributes, key.secret_scalar().as_ref())
                }
                SecretKeyType::Aes256 => {
                    let mut key = [0u8; 32];
                    rng.fill_bytes(&mut key);
                    swkey_insert(attributes, key.as_ref())
                }
                SecretKeyType::Aes128 => {
                    let mut key = [0u8; 16];
                    rng.fill_bytes(&mut key);
                    swkey_insert(attributes, key.as_ref())
                }
                SecretKeyType::Buffer(size) => {
                    let mut key = vec![0u8; size];
                    rng.fill_bytes(key.as_mut_slice());
                    swkey_insert(attributes, key.as_slice())
                }
            },
        }
    }

    fn secret_import(
        &mut self,
        secret: &SecretKey,
        attributes: SecretKeyAttributes,
    ) -> Result<SecretKeyContext, VaultFailError> {
        let mut swkey_insert = |atts: SecretKeyAttributes,
                                buffer: &[u8]|
         -> Result<SecretKeyContext, VaultFailError> {
            let mut r = rand::rngs::OsRng {};
            let id = r.gen::<usize>();
            let mut bytes = atts.to_bytes().to_vec();
            bytes.extend_from_slice(buffer);
            self.unlock()?;
            self.keychain.set_generic_password(
                OCKAM_SERVICE_NAME,
                id.to_string().as_str(),
                bytes.as_slice(),
            )?;

            Ok(SecretKeyContext::KeyRing {
                id,
                os_type: OsKeyRing::Osx(OsxContext::Keychain),
            })
        };
        match attributes.persistence {
            SecretPersistenceType::Ephemeral => self
                .ephemeral_vault
                .secret_import(secret, attributes)
                .map(|c| {
                    if let SecretKeyContext::Memory(id) = c {
                        SecretKeyContext::KeyRing {
                            id,
                            os_type: OsKeyRing::Osx(OsxContext::Memory),
                        }
                    } else {
                        c
                    }
                }),
            SecretPersistenceType::Persistent => swkey_insert(attributes, secret.as_ref()),
        }
    }

    fn secret_export(&mut self, context: SecretKeyContext) -> Result<SecretKey, VaultFailError> {
        if let SecretKeyContext::KeyRing { id, os_type } = context {
            if let OsKeyRing::Osx(ctx) = os_type {
                return match ctx {
                    OsxContext::Memory => {
                        let cid = SecretKeyContext::Memory(id);
                        self.ephemeral_vault.secret_export(cid)
                    }
                    OsxContext::Keychain => {
                        self.unlock()?;
                        let (key, _) = self
                            .keychain
                            .find_generic_password(OCKAM_SERVICE_NAME, id.to_string().as_str())?;
                        let bytes = key.to_owned();
                        let mut atts = [0u8; 6];
                        atts.copy_from_slice(&bytes[0..6]);
                        let attributes = SecretKeyAttributes::try_from(atts)?;
                        Ok(match attributes.xtype {
                            SecretKeyType::Buffer(_) => SecretKey::Buffer(bytes[6..].to_vec()),
                            SecretKeyType::P256 => SecretKey::P256(*array_ref![bytes, 6, 32]),
                            SecretKeyType::Curve25519 => {
                                SecretKey::Curve25519(*array_ref![bytes, 6, 32])
                            }
                            SecretKeyType::Aes128 => SecretKey::Aes128(*array_ref![bytes, 6, 16]),
                            SecretKeyType::Aes256 => SecretKey::Aes256(*array_ref![bytes, 6, 32]),
                        })
                    }
                    OsxContext::Enclave => Err(VaultFailErrorKind::AccessDenied.into()),
                };
            } else {
                Err(VaultFailErrorKind::InvalidContext.into())
            }
        } else {
            Err(VaultFailErrorKind::InvalidContext.into())
        }
    }

    fn secret_attributes_get(
        &mut self,
        context: SecretKeyContext,
    ) -> Result<SecretKeyAttributes, VaultFailError> {
        if let SecretKeyContext::KeyRing { id, os_type } = context {
            if let OsKeyRing::Osx(ctx) = os_type {
                return match ctx {
                    OsxContext::Memory => {
                        let cid = SecretKeyContext::Memory(id);
                        self.ephemeral_vault.secret_attributes_get(cid)
                    }
                    OsxContext::Enclave => Ok(SecretKeyAttributes {
                        xtype: SecretKeyType::P256,
                        persistence: SecretPersistenceType::Persistent,
                        purpose: SecretPurposeType::KeyAgreement,
                    }),
                    OsxContext::Keychain => {
                        self.unlock()?;
                        let (key, _) = self
                            .keychain
                            .find_generic_password(OCKAM_SERVICE_NAME, id.to_string().as_str())?;
                        let bytes = key.to_owned();
                        let mut atts = [0u8; 6];
                        atts.copy_from_slice(&bytes[..6]);
                        let attributes = SecretKeyAttributes::try_from(atts)?;
                        Ok(attributes)
                    }
                };
            } else {
                Err(VaultFailErrorKind::InvalidContext.into())
            }
        } else {
            Err(VaultFailErrorKind::InvalidContext.into())
        }
    }

    fn secret_public_key_get(
        &mut self,
        context: SecretKeyContext,
    ) -> Result<PublicKey, VaultFailError> {
        if let SecretKeyContext::KeyRing { id, os_type } = context {
            if let OsKeyRing::Osx(ctx) = os_type {
                match ctx {
                    OsxContext::Memory => {
                        let cid = SecretKeyContext::Memory(id);
                        self.ephemeral_vault.secret_public_key_get(cid)
                    },
                    OsxContext::Keychain => {
                        self.unlock()?;
                        let (key, _) = self
                            .keychain
                            .find_generic_password(OCKAM_SERVICE_NAME, id.to_string().as_str())?;
                        let bytes = key.to_owned();
                        let mut atts = [0u8; 6];
                        atts.copy_from_slice(&bytes[..6]);
                        let attributes = SecretKeyAttributes::try_from(atts)?;
                        let key = *array_ref![bytes, 6, 32];
                        match attributes.xtype {
                            SecretKeyType::Curve25519 => {
                                let sk = x25519_dalek::StaticSecret::from(key);
                                let pk = x25519_dalek::PublicKey::from(&sk);
                                Ok(PublicKey::Curve25519(*pk.as_bytes()))
                            },
                            SecretKeyType::P256 => {
                                let sk = Scalar::from_bytes(key).unwrap();
                                let pp = ProjectivePoint::generator() * &sk;
                                let pk = p256::elliptic_curve::weierstrass::PublicKey::from(
                                    pp.to_affine().unwrap().to_uncompressed_pubkey(),
                                );
                                Ok(PublicKey::P256(*array_ref![pk.as_bytes(), 0, 65]))
                            },
                            _ => Err(VaultFailErrorKind::PublicKey.into()),
                        }
                    },
                    _ => Err(VaultFailErrorKind::PublicKey.into()),
                }
            } else {
                Err(VaultFailErrorKind::InvalidContext.into())
            }
        } else {
            Err(VaultFailErrorKind::InvalidContext.into())
        }
    }

    fn secret_destroy(&mut self, context: SecretKeyContext) -> Result<(), VaultFailError> {
        if let SecretKeyContext::KeyRing { id, os_type } = context {
            if let OsKeyRing::Osx(ctx) = os_type {
                match ctx {
                    OsxContext::Memory => {
                        let memctx = SecretKeyContext::Memory(id);
                        self.ephemeral_vault.secret_destroy(memctx)
                    }
                    OsxContext::Keychain => {
                        self.unlock()?;
                        let (_, item) = self
                            .keychain
                            .find_generic_password(OCKAM_SERVICE_NAME, id.to_string().as_str())?;
                        item.delete();
                        Ok(())
                    }
                    OsxContext::Enclave => {
                        let label = format!("{}-{}", OCKAM_SERVICE_NAME, id);
                        let query = enclave::item::Query::new().label(label.as_str());
                        let key = enclave::key::Key::find(query)?;
                        key.delete()?;
                        Ok(())
                    }
                }
            } else {
                Err(VaultFailErrorKind::InvalidParam(0).into())
            }
        } else {
            Err(VaultFailErrorKind::InvalidParam(0).into())
        }
    }

    fn ec_diffie_hellman(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
    ) -> Result<SecretKeyContext, VaultFailError> {
        if let SecretKeyContext::KeyRing { id, os_type } = context {
            if let OsKeyRing::Osx(ctx) = os_type {
                match ctx {
                    OsxContext::Memory => {
                        let cid = SecretKeyContext::Memory(id);
                        self.ephemeral_vault.ec_diffie_hellman(cid, peer_public_key)
                    },
                    OsxContext::Keychain => {
                        self.unlock()?;
                        let (key, _) = self
                            .keychain
                            .find_generic_password(OCKAM_SERVICE_NAME, id.to_string().as_str())?;
                        let bytes = key.to_owned();
                        let mut atts = [0u8; 6];
                        atts.copy_from_slice(&bytes[..6]);
                        let attributes = SecretKeyAttributes::try_from(atts)?;
                        let key = *array_ref![bytes, 6, 32];
                        match (attributes.xtype, peer_public_key) {
                            (SecretKeyType::Curve25519, PublicKey::Curve25519(b)) => {
                                let sk = x25519_dalek::StaticSecret::from(key);
                                let pk_t = x25519_dalek::PublicKey::from(b);
                                let secret = sk.diffie_hellman(&pk_t);
                                let buffer = SecretKey::Buffer(secret.as_bytes().to_vec());
                                let attributes = SecretKeyAttributes {
                                    xtype: SecretKeyType::Buffer(32),
                                    purpose: SecretPurposeType::KeyAgreement,
                                    persistence: SecretPersistenceType::Ephemeral,
                                };
                                self.ephemeral_vault.secret_import(&buffer, attributes)
                            },
                            (SecretKeyType::P256, PublicKey::P256(b)) => {
                                let o_pk_t: Option<p256::elliptic_curve::weierstrass::PublicKey<p256::NistP256>> =
                                    p256::elliptic_curve::weierstrass::PublicKey::from_bytes(b.as_ref());
                                if o_pk_t.is_none() {
                                    fail!(VaultFailErrorKind::Ecdh);
                                }
                                let pk_t = o_pk_t.unwrap();
                                let o_p_t = AffinePoint::from_pubkey(&pk_t);
                                if o_p_t.is_none().unwrap_u8() == 1 {
                                    fail!(VaultFailErrorKind::Ecdh);
                                }
                                let sk = Scalar::from_bytes(key).unwrap();
                                let pk_t = ProjectivePoint::from(o_p_t.unwrap());
                                let secret = &pk_t * &sk;
                                if secret.ct_eq(&ProjectivePoint::identity()).unwrap_u8() == 1 {
                                    fail!(VaultFailErrorKind::Ecdh);
                                }
                                let result = secret.to_affine().unwrap().to_compressed_pubkey();
                                // Throw away the compressed indicator byte
                                let buffer = SecretKey::Buffer(result.as_ref()[1..].to_vec());
                                let attributes = SecretKeyAttributes {
                                    xtype: SecretKeyType::Buffer(32),
                                    purpose: SecretPurposeType::KeyAgreement,
                                    persistence: SecretPersistenceType::Ephemeral,
                                };
                                self.ephemeral_vault.secret_import(&buffer, attributes)
                            },
                            _ => Err(VaultFailErrorKind::PublicKey.into()),
                        }
                    },
                    _ => Err(VaultFailErrorKind::PublicKey.into()),
                }
            } else {
                Err(VaultFailErrorKind::InvalidContext.into())
            }
        } else {
            Err(VaultFailErrorKind::InvalidContext.into())
        }
    }

    fn ec_diffie_hellman_hkdf_sha256<B: AsRef<[u8]>>(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
        salt: B,
        okm_len: usize,
    ) -> Result<Vec<u8>, VaultFailError> {
        let ctx = self.ec_diffie_hellman(context, peer_public_key)?;
        let shared_secret = self.secret_export(ctx)?;
        let mut okm = vec![0u8; okm_len];
        let prk = hkdf::Hkdf::<Sha256>::new(Some(salt.as_ref()), &shared_secret.as_ref());
        prk.expand(b"", okm.as_mut_slice())?;
        Ok(okm)
    }

    fn hkdf_sha256<B: AsRef<[u8]>, C: AsRef<[u8]>>(
        &mut self,
        salt: B,
        ikm: C,
        okm_len: usize,
    ) -> Result<Vec<u8>, VaultFailError> {
        self.ephemeral_vault.hkdf_sha256(salt, ikm, okm_len)
    }

    fn aead_aes_gcm_encrypt<B: AsRef<[u8]>, C: AsRef<[u8]>, D: AsRef<[u8]>>(
        &mut self,
        _context: SecretKeyContext,
        _plaintext: B,
        _nonce: C,
        _aad: D,
    ) -> Result<Vec<u8>, VaultFailError> {
        unimplemented!()
    }

    fn aead_aes_gcm_decrypt<B: AsRef<[u8]>, C: AsRef<[u8]>, D: AsRef<[u8]>>(
        &mut self,
        _context: SecretKeyContext,
        _cipher_text: B,
        _nonce: C,
        _aad: D,
    ) -> Result<Vec<u8>, VaultFailError> {
        unimplemented!()
    }

    fn deinit(&mut self) {
        self.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SecretPurposeType;

    #[test]
    fn new_vault() {
        let mut vault = OsxVault::default();
        // Panics if the default keychain and SEP are not available
        vault.unlock().unwrap();
    }

    #[test]
    fn new_secret_keys() {
        let mut vault = OsxVault::default();
        let mut attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Curve25519,
            persistence: SecretPersistenceType::Persistent,
            purpose: SecretPurposeType::KeyAgreement,
        };
        let types = [
            SecretKeyType::Curve25519,
            SecretKeyType::Aes128,
            SecretKeyType::Aes256,
            SecretKeyType::Buffer(24),
        ];
        for t in &types {
            attributes.xtype = *t;
            let res = vault.secret_generate(attributes);
            assert!(res.is_ok());
            let ctx = res.unwrap();
            let res = vault.secret_destroy(ctx);
            assert!(res.is_ok());
        }
    }

    #[ignore]
    #[test]
    fn new_enclave_keys() {
        let mut vault = OsxVault::default();
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::P256,
            persistence: SecretPersistenceType::Persistent,
            purpose: SecretPurposeType::KeyAgreement,
        };
        let res = vault.secret_generate(attributes);
        assert!(res.is_ok());
        let ctx = res.unwrap();
        let res = vault.secret_destroy(ctx);
        assert!(res.is_ok());
    }
}
