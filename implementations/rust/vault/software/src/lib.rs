use crate::error::*;
use crate::xeddsa::*;
use aead::{generic_array::GenericArray, Aead, NewAead, Payload};
use aes_gcm::{Aes128Gcm, Aes256Gcm};
use ockam_vault::{
    error::{VaultFailError, VaultFailErrorKind},
    types::*,
    AsymmetricVault, HashVault, Secret, SecretVault, SignerVault, SymmetricVault, VerifierVault,
};
use p256::{
    elliptic_curve::{sec1::FromEncodedPoint, Group},
    AffinePoint, ProjectivePoint, Scalar,
};
use rand::{prelude::*, rngs::OsRng};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use zeroize::Zeroize;

pub extern crate ockam_vault;

mod xeddsa;

mod error;

#[macro_use]
extern crate ockam_common;
#[macro_use]
extern crate arrayref;

/// Default vault secret
#[derive(Debug, Copy, Clone)]
pub struct DefaultVaultSecret(usize);

impl DefaultVaultSecret {
    pub fn downcast_secret(
        context: &Box<dyn Secret>,
    ) -> Result<&DefaultVaultSecret, VaultFailError> {
        context
            .downcast_ref::<DefaultVaultSecret>()
            .map_err(|_| VaultFailErrorKind::InvalidSecret.into())
    }
}

impl Zeroize for DefaultVaultSecret {
    fn zeroize(&mut self) {}
}

impl Secret for DefaultVaultSecret {}

/// A pure rust implementation of a vault.
/// Is not thread-safe i.e. if multiple threads
/// add values to the vault there may be collisions
/// This is mostly for testing purposes anyway
/// and shouldn't be used for production
///
/// ```
/// use ockam_vault_software::DefaultVault;
/// let vault = DefaultVault::default();
/// ```
#[derive(Debug)]
pub struct DefaultVault {
    entries: BTreeMap<usize, VaultEntry>,
    next_id: usize,
}

impl Default for DefaultVault {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            next_id: 0,
        }
    }
}

impl DefaultVault {
    fn get_entry(
        &self,
        context: &Box<dyn Secret>,
        error: VaultFailErrorKind,
    ) -> Result<&VaultEntry, VaultFailError> {
        let id = DefaultVaultSecret::downcast_secret(context)?.0;

        let entry;
        if let Some(e) = self.entries.get(&id) {
            entry = e;
        } else {
            fail!(error);
        }
        Ok(entry)
    }

    fn ecdh_internal(
        vault_entry: &VaultEntry,
        peer_public_key: &[u8],
    ) -> Result<Vec<u8>, VaultFailError> {
        let key = vault_entry.key.as_ref();
        match vault_entry.key_attributes.stype {
            SecretType::Curve25519
                if peer_public_key.len() == CURVE25519_PUBLIC_LENGTH
                    && key.len() == CURVE25519_SECRET_LENGTH =>
            {
                let sk =
                    x25519_dalek::StaticSecret::from(*array_ref!(key, 0, CURVE25519_SECRET_LENGTH));
                let pk_t = x25519_dalek::PublicKey::from(*array_ref!(
                    peer_public_key,
                    0,
                    CURVE25519_PUBLIC_LENGTH
                ));
                let secret = sk.diffie_hellman(&pk_t);
                Ok(secret.as_bytes().to_vec())
            }
            SecretType::P256
                if peer_public_key.len() == P256_PUBLIC_LENGTH
                    && key.len() == P256_SECRET_LENGTH =>
            {
                let o_pk_t = p256::elliptic_curve::sec1::EncodedPoint::from_bytes(peer_public_key);
                if o_pk_t.is_err() {
                    fail!(VaultFailErrorKind::Ecdh);
                }
                let pk_t = o_pk_t.unwrap();
                let o_p_t = AffinePoint::from_encoded_point(&pk_t);
                if o_p_t.is_none().unwrap_u8() == 1 {
                    fail!(VaultFailErrorKind::Ecdh);
                }
                let sk = Scalar::from_bytes_reduced(p256::FieldBytes::from_slice(key));
                let pk_t = ProjectivePoint::from(o_p_t.unwrap());
                let secret = pk_t * sk;
                if secret.is_identity().unwrap_u8() == 1 {
                    fail!(VaultFailErrorKind::Ecdh);
                }
                let ap = p256::elliptic_curve::sec1::EncodedPoint::from(secret.to_affine());
                Ok(ap.x().as_slice().to_vec())
            }
            _ => Err(VaultFailError::from_msg(
                VaultFailErrorKind::Ecdh,
                "Unknown key type",
            )),
        }
    }

    fn hkdf_sha256_internal(
        &mut self,
        salt: &Box<dyn Secret>,
        info: &[u8],
        ikm: &[u8],
        output_attributes: Vec<SecretAttributes>,
    ) -> Result<Vec<Box<dyn Secret>>, VaultFailError> {
        let salt = self.get_entry(salt, VaultFailErrorKind::Ecdh)?;

        // FIXME: Doesn't work for secrets with size more than 32 bytes
        let okm_len = output_attributes.len() * 32;

        let okm = {
            let mut okm = vec![0u8; okm_len];
            let prk = hkdf::Hkdf::<Sha256>::new(Some(salt.key.as_ref()), ikm);
            prk.expand(info, okm.as_mut_slice())
                .map_err(map_hkdf_invalid_length_err)?;
            okm
        };

        let mut secrets = Vec::<Box<dyn Secret>>::new();
        let mut index = 0;

        for attributes in output_attributes {
            let length = attributes.length;
            if attributes.stype == SecretType::Aes {
                if length != AES256_SECRET_LENGTH && length != AES128_SECRET_LENGTH {
                    return Err(VaultFailError::from_msg(
                        VaultFailErrorKind::HkdfSha256,
                        "Unknown key type",
                    ));
                }
            } else if attributes.stype != SecretType::Buffer {
                return Err(VaultFailError::from_msg(
                    VaultFailErrorKind::HkdfSha256,
                    "Unknown key type",
                ));
            }
            let secret = &okm[index..index + length];
            let secret = self.secret_import(&secret, attributes)?;

            secrets.push(secret);
            index += 32;
        }

        Ok(secrets)
    }

    pub fn get_ids(&self) -> Vec<usize> {
        self.entries.keys().map(|i| *i).collect()
    }
}

impl Zeroize for DefaultVault {
    fn zeroize(&mut self) {
        for (_, v) in self.entries.iter_mut() {
            v.zeroize();
        }
        self.entries.clear();
        self.next_id = 0;
    }
}

zdrop_impl!(DefaultVault);

#[derive(Debug, Eq, PartialEq)]
struct VaultEntry {
    id: usize,
    key_attributes: SecretAttributes,
    key: SecretKey,
}

impl Zeroize for VaultEntry {
    fn zeroize(&mut self) {
        self.key.zeroize()
    }
}

zdrop_impl!(VaultEntry);

macro_rules! encrypt_op_impl {
    ($a:expr,$aad:expr,$nonce:expr,$text:expr,$type:ident,$op:ident) => {{
        let key = GenericArray::from_slice($a.as_ref());
        let cipher = $type::new(key);
        let nonce = GenericArray::from_slice($nonce.as_ref());
        let payload = Payload {
            aad: $aad.as_ref(),
            msg: $text.as_ref(),
        };
        let output = cipher.$op(nonce, payload).map_err(map_aes_error)?;
        Ok(output)
    }};
}

macro_rules! encrypt_impl {
    ($entry:expr, $aad:expr, $nonce: expr, $text:expr, $op:ident, $err:expr) => {{
        if $entry.key_attributes.stype != SecretType::Aes {
            return Err($err.into());
        }
        match $entry.key_attributes.length {
            AES128_SECRET_LENGTH => {
                encrypt_op_impl!($entry.key.as_ref(), $aad, $nonce, $text, Aes128Gcm, $op)
            }
            AES256_SECRET_LENGTH => {
                encrypt_op_impl!($entry.key.as_ref(), $aad, $nonce, $text, Aes256Gcm, $op)
            }
            _ => Err($err.into()),
        }
    }};
}

impl SecretVault for DefaultVault {
    fn secret_generate(
        &mut self,
        attributes: SecretAttributes,
    ) -> Result<Box<dyn Secret>, VaultFailError> {
        let mut rng = OsRng {};
        let length = attributes.length;
        let key = match attributes.stype {
            SecretType::Curve25519 => {
                let sk = x25519_dalek::StaticSecret::new(&mut rng);
                SecretKey::new(sk.to_bytes().to_vec())
            }
            SecretType::Aes => {
                if length != AES256_SECRET_LENGTH && length != AES128_SECRET_LENGTH {
                    return Err(VaultFailError::from_msg(
                        VaultFailErrorKind::HkdfSha256,
                        "Unknown key type",
                    ));
                };
                let mut key = vec![0u8; length];
                rng.fill_bytes(&mut key);
                SecretKey::new(key)
            }
            SecretType::P256 => {
                let key = p256::SecretKey::random(&mut rng);
                let mut value = [0u8; 32];
                value.copy_from_slice(&key.secret_scalar().to_bytes());
                SecretKey::new(value.to_vec())
            }
            SecretType::Buffer => {
                let mut key = vec![0u8; attributes.length];
                rng.fill_bytes(key.as_mut_slice());
                SecretKey::new(key)
            }
        };
        self.next_id += 1;
        self.entries.insert(
            self.next_id,
            VaultEntry {
                id: self.next_id,
                key_attributes: attributes,
                key,
            },
        );

        Ok(Box::new(DefaultVaultSecret(self.next_id)))
    }

    fn secret_import(
        &mut self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> Result<Box<dyn Secret>, VaultFailError> {
        self.next_id += 1;
        self.entries.insert(
            self.next_id,
            VaultEntry {
                id: self.next_id,
                key_attributes: attributes,
                key: SecretKey::new(secret.to_vec()),
            },
        );
        Ok(Box::new(DefaultVaultSecret(self.next_id)))
    }

    fn secret_export(&mut self, context: &Box<dyn Secret>) -> Result<SecretKey, VaultFailError> {
        self.get_entry(context, VaultFailErrorKind::InvalidSecret)
            .map(|i| i.key.clone())
    }

    fn secret_attributes_get(
        &mut self,
        context: &Box<dyn Secret>,
    ) -> Result<SecretAttributes, VaultFailError> {
        self.get_entry(context, VaultFailErrorKind::InvalidSecret)
            .map(|i| i.key_attributes)
    }

    fn secret_public_key_get(
        &mut self,
        context: &Box<dyn Secret>,
    ) -> Result<PublicKey, VaultFailError> {
        let entry = self.get_entry(context, VaultFailErrorKind::PublicKey)?;

        match entry.key_attributes.stype {
            SecretType::Curve25519 => {
                if entry.key.as_ref().len() != CURVE25519_SECRET_LENGTH {
                    return Err(VaultFailErrorKind::PublicKey.into());
                }
                let sk = x25519_dalek::StaticSecret::from(*array_ref![
                    entry.key.as_ref(),
                    0,
                    CURVE25519_SECRET_LENGTH
                ]);
                let pk = x25519_dalek::PublicKey::from(&sk);
                Ok(PublicKey::new(pk.to_bytes().to_vec()))
            }
            SecretType::P256 => {
                let sk =
                    Scalar::from_bytes_reduced(p256::FieldBytes::from_slice(entry.key.as_ref()));
                let pp = ProjectivePoint::generator() * sk;
                let ap = p256::elliptic_curve::sec1::EncodedPoint::from(pp.to_affine());
                Ok(PublicKey::new(ap.as_bytes().to_vec()))
            }
            _ => Err(VaultFailErrorKind::PublicKey.into()),
        }
    }

    fn secret_destroy(&mut self, context: Box<dyn Secret>) -> Result<(), VaultFailError> {
        let id = DefaultVaultSecret::downcast_secret(&context)?.0;
        if let Some(mut k) = self.entries.remove(&id) {
            k.key.zeroize();
        }
        Ok(())
    }
}

impl HashVault for DefaultVault {
    fn sha256(&self, data: &[u8]) -> Result<[u8; 32], VaultFailError> {
        let digest = Sha256::digest(data);
        Ok(*array_ref![digest, 0, 32])
    }

    fn hkdf_sha256(
        &mut self,
        salt: &Box<dyn Secret>,
        info: &[u8],
        ikm: Option<&Box<dyn Secret>>,
        output_attributes: Vec<SecretAttributes>,
    ) -> Result<Vec<Box<dyn Secret>>, VaultFailError> {
        let ikm_slice = match ikm {
            Some(ikm) => {
                let ikm = self.get_entry(ikm, VaultFailErrorKind::HkdfSha256)?;
                if ikm.key_attributes.stype == SecretType::Buffer {
                    Ok(ikm.key.as_ref().to_vec())
                } else {
                    Err(VaultFailError::from_msg(
                        VaultFailErrorKind::HkdfSha256,
                        "Unknown key type",
                    ))
                }
            }
            None => Ok(Vec::new()),
        }?;

        self.hkdf_sha256_internal(salt, info, &ikm_slice, output_attributes)
    }
}

impl AsymmetricVault for DefaultVault {
    fn ec_diffie_hellman(
        &mut self,
        context: &Box<dyn Secret>,
        peer_public_key: &[u8],
    ) -> Result<Box<dyn Secret>, VaultFailError> {
        let entry = self.get_entry(context, VaultFailErrorKind::Ecdh)?;

        let dh = Self::ecdh_internal(entry, peer_public_key)?;

        let attributes = SecretAttributes {
            stype: SecretType::Buffer,
            persistence: SecretPersistence::Ephemeral,
            length: dh.len(),
        };
        self.secret_import(&dh, attributes)
    }
}

impl SymmetricVault for DefaultVault {
    fn aead_aes_gcm_encrypt(
        &mut self,
        context: &Box<dyn Secret>,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, VaultFailError> {
        let entry = self.get_entry(context, VaultFailErrorKind::AeadAesGcmEncrypt)?;

        encrypt_impl!(
            entry,
            aad,
            nonce,
            plaintext,
            encrypt,
            VaultFailErrorKind::AeadAesGcmEncrypt
        )
    }

    fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Box<dyn Secret>,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, VaultFailError> {
        let entry = self.get_entry(context, VaultFailErrorKind::AeadAesGcmDecrypt)?;
        encrypt_impl!(
            entry,
            aad,
            nonce,
            cipher_text,
            decrypt,
            VaultFailErrorKind::AeadAesGcmDecrypt
        )
    }
}

impl SignerVault for DefaultVault {
    fn sign(
        &mut self,
        secret_key: &Box<dyn Secret>,
        data: &[u8],
    ) -> Result<[u8; 64], VaultFailError> {
        let entry = self.get_entry(secret_key, VaultFailErrorKind::Ecdh)?;
        let key = entry.key.as_ref();
        match entry.key_attributes.stype {
            SecretType::Curve25519 if key.len() == CURVE25519_SECRET_LENGTH => {
                let mut rng = thread_rng();
                let mut nonce = [0u8; 64];
                rng.fill_bytes(&mut nonce);
                let sig =
                    x25519_dalek::StaticSecret::from(*array_ref!(key, 0, CURVE25519_SECRET_LENGTH))
                        .sign(data.as_ref(), &nonce);
                Ok(sig)
            }
            // SecretKey::P256(k) => {
            //     let sign_key = SigningKey::new(&k)?;
            //     let sig = sign_key.sign(data.as_ref());
            //     Ok(*array_ref![sig.as_ref(), 0, 64])
            // }
            _ => Err(VaultFailError::from_msg(
                VaultFailErrorKind::Ecdh,
                "Unhandled key type",
            )),
        }
    }
}

impl VerifierVault for DefaultVault {
    fn verify(
        &mut self,
        signature: &[u8; 64],
        public_key: &[u8],
        data: &[u8],
    ) -> Result<(), VaultFailError> {
        // FIXME
        if public_key.len() == CURVE25519_PUBLIC_LENGTH {
            if x25519_dalek::PublicKey::from(*array_ref!(public_key, 0, CURVE25519_PUBLIC_LENGTH))
                .verify(data.as_ref(), &signature)
            {
                Ok(())
            } else {
                Err(VaultFailErrorKind::PublicKey.into())
            }
        } else {
            Err(VaultFailErrorKind::PublicKey.into())
        }
        // match public_key {
        // PublicKey::P256(k) => {
        //     let key = VerifyKey::new(&k)?;
        //     let sig = Signature::try_from(&signature[..])?;
        //     key.verify(data.as_ref(), &sig)?;
        //     Ok(())
        // }
        // }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_vault() {
        let vault = DefaultVault::default();
        assert_eq!(vault.next_id, 0);
        assert_eq!(vault.entries.len(), 0);
    }

    #[test]
    fn new_public_keys() {
        let mut vault = DefaultVault::default();
        let mut attributes = SecretAttributes {
            stype: SecretType::P256,
            persistence: SecretPersistence::Ephemeral,
            length: P256_SECRET_LENGTH,
        };

        let res = vault.secret_generate(attributes);
        assert!(res.is_ok());
        let p256_ctx_1 = res.unwrap();

        let res = vault.secret_public_key_get(&p256_ctx_1);
        assert!(res.is_ok());
        let pk_1 = res.unwrap();
        assert_eq!(pk_1.as_ref().len(), P256_PUBLIC_LENGTH);
        assert_eq!(vault.entries.len(), 1);
        assert_eq!(vault.next_id, 1);

        attributes.stype = SecretType::Curve25519;

        let res = vault.secret_generate(attributes);
        assert!(res.is_ok());
        let c25519_ctx_1 = res.unwrap();
        let res = vault.secret_public_key_get(&c25519_ctx_1);
        assert!(res.is_ok());
        let pk_1 = res.unwrap();
        assert_eq!(pk_1.as_ref().len(), CURVE25519_PUBLIC_LENGTH);
        assert_eq!(vault.entries.len(), 2);
        assert_eq!(vault.next_id, 2);
    }

    #[test]
    fn new_secret_keys() {
        let mut vault = DefaultVault::default();
        let mut attributes = SecretAttributes {
            stype: SecretType::P256,
            persistence: SecretPersistence::Ephemeral,
            length: P256_SECRET_LENGTH,
        };
        let types = [
            (SecretType::Curve25519, 32),
            (SecretType::P256, 32),
            (SecretType::Aes, 32),
            (SecretType::Aes, 16),
            (SecretType::Buffer, 24),
        ];
        for (t, s) in &types {
            attributes.stype = *t;
            attributes.length = *s;
            let res = vault.secret_generate(attributes);
            assert!(res.is_ok());
            let sk_ctx = res.unwrap();
            let sk = vault.secret_export(&sk_ctx).unwrap();
            assert_eq!(sk.as_ref().len(), *s);
            vault.secret_destroy(sk_ctx).unwrap();
            assert_eq!(vault.entries.len(), 0);
        }
    }

    #[test]
    fn sha256() {
        let vault = DefaultVault::default();
        let res = vault.sha256(b"a");
        assert!(res.is_ok());
        let digest = res.unwrap();
        assert_eq!(
            hex::encode(digest),
            "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
        );
    }

    #[test]
    fn hkdf() {
        let mut vault = DefaultVault::default();

        let salt_value = b"hkdf_test";
        let attributes = SecretAttributes {
            stype: SecretType::Buffer,
            persistence: SecretPersistence::Ephemeral,
            length: salt_value.len(),
        };
        let salt = vault.secret_import(&salt_value[..], attributes).unwrap();

        let ikm_value = b"a";
        let attributes = SecretAttributes {
            stype: SecretType::Buffer,
            persistence: SecretPersistence::Ephemeral,
            length: ikm_value.len(),
        };
        let ikm = vault.secret_import(&ikm_value[..], attributes).unwrap();

        let attributes = SecretAttributes {
            stype: SecretType::Buffer,
            persistence: SecretPersistence::Ephemeral,
            length: 24,
        };

        let res = vault.hkdf_sha256(&salt, b"", Some(&ikm), vec![attributes]);
        assert!(res.is_ok());
        let digest = res.unwrap();
        assert_eq!(digest.len(), 1);
        let digest = vault.secret_export(&digest[0]).unwrap();
        assert_eq!(
            hex::encode(digest.as_ref()),
            "921ab9f260544b71941dbac2ca2d42c417aa07b53e055a8f"
        );
    }

    #[test]
    fn ec_diffie_hellman_p256() {
        let mut vault = DefaultVault::default();
        let attributes = SecretAttributes {
            stype: SecretType::P256,
            persistence: SecretPersistence::Ephemeral,
            length: P256_SECRET_LENGTH,
        };
        let sk_ctx_1 = vault.secret_generate(attributes).unwrap();
        let sk_ctx_2 = vault.secret_generate(attributes).unwrap();
        let pk_1 = vault.secret_public_key_get(&sk_ctx_1).unwrap();
        let pk_2 = vault.secret_public_key_get(&sk_ctx_2).unwrap();

        let res = vault.ec_diffie_hellman(&sk_ctx_1, pk_2.as_ref());
        assert!(res.is_ok());
        let ss = res.unwrap();
        // TODO: Check result against test vector
    }

    #[test]
    fn ec_diffie_hellman_curve25519() {
        let mut vault = DefaultVault::default();
        let attributes = SecretAttributes {
            stype: SecretType::Curve25519,
            persistence: SecretPersistence::Ephemeral,
            length: CURVE25519_SECRET_LENGTH,
        };
        let sk_ctx_1 = vault.secret_generate(attributes).unwrap();
        let sk_ctx_2 = vault.secret_generate(attributes).unwrap();
        let pk_1 = vault.secret_public_key_get(&sk_ctx_1).unwrap();
        let pk_2 = vault.secret_public_key_get(&sk_ctx_2).unwrap();

        let res = vault.ec_diffie_hellman(&sk_ctx_1, pk_2.as_ref());
        assert!(res.is_ok());
        let ss = res.unwrap();
        // TODO: Check result against test vector
    }

    #[test]
    fn ec_diffie_hellman_different_keys() {
        let mut vault = DefaultVault::default();
        let mut attributes = SecretAttributes {
            stype: SecretType::Curve25519,
            persistence: SecretPersistence::Ephemeral,
            length: CURVE25519_SECRET_LENGTH,
        };
        let sk_ctx_1 = vault.secret_generate(attributes).unwrap();
        let pk_1 = vault.secret_public_key_get(&sk_ctx_1).unwrap();
        attributes.stype = SecretType::P256;
        let sk_ctx_2 = vault.secret_generate(attributes).unwrap();
        let pk_2 = vault.secret_public_key_get(&sk_ctx_2).unwrap();

        let res = vault.ec_diffie_hellman(&sk_ctx_1, pk_2.as_ref());
        assert!(res.is_err());
        let res = vault.ec_diffie_hellman(&sk_ctx_2, pk_1.as_ref());
        assert!(res.is_err());
    }

    #[test]
    fn encryption() {
        let mut vault = DefaultVault::default();
        let message = b"Ockam Test Message";
        let nonce = b"TestingNonce";
        let aad = b"Extra payload data";
        let attributes = SecretAttributes {
            stype: SecretType::Aes,
            persistence: SecretPersistence::Ephemeral,
            length: AES128_SECRET_LENGTH,
        };

        let ctx = &vault.secret_generate(attributes).unwrap();
        let res = vault.aead_aes_gcm_encrypt(ctx, message.as_ref(), nonce.as_ref(), aad.as_ref());
        assert!(res.is_ok());
        let mut ciphertext = res.unwrap();
        let res =
            vault.aead_aes_gcm_decrypt(ctx, ciphertext.as_slice(), nonce.as_ref(), aad.as_ref());
        assert!(res.is_ok());
        let plaintext = res.unwrap();
        assert_eq!(plaintext, message.to_vec());
        ciphertext[0] ^= ciphertext[1];
        let res =
            vault.aead_aes_gcm_decrypt(ctx, ciphertext.as_slice(), nonce.as_ref(), aad.as_ref());
        assert!(res.is_err());
    }

    #[test]
    fn sign() {
        let mut vault = DefaultVault::default();
        let secret = vault
            .secret_generate(SecretAttributes {
                persistence: SecretPersistence::Ephemeral,
                stype: SecretType::Curve25519,
                length: CURVE25519_SECRET_LENGTH,
            })
            .unwrap();
        let res = vault.sign(&secret, b"hello world!");
        assert!(res.is_ok());
        let pubkey = vault.secret_public_key_get(&secret).unwrap();
        let signature = res.unwrap();
        let res = vault.verify(&signature, pubkey.as_ref(), b"hello world!");
        assert!(res.is_ok());
    }
}
