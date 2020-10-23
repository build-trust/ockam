
#[cfg(feature = "nostd-stm32f4")]
use alloc::vec::Vec;

#[cfg(not(feature = "nostd-stm32f4"))]
type VaultMap = std::collections::BTreeMap<usize, VaultEntry>;

#[cfg(feature = "nostd-stm32f4")]
type VaultMap = hashbrown::HashMap<usize, VaultEntry>;

#[cfg(feature = "nostd-stm32f4")]
use crate::alloc::borrow::ToOwned;
// use core::marker::Copy;

use crate::{
    error::{VaultFailError, VaultFailErrorKind},
    types::*,
    Vault,
};
use aead::{generic_array::GenericArray, Aead, NewAead, Payload};
use aes_gcm::{Aes128Gcm, Aes256Gcm};
use p256::{
    elliptic_curve::{sec1::FromEncodedPoint, Group},
    AffinePoint, ProjectivePoint, Scalar,
};



use sha2::{Digest, Sha256};

use xeddsa::*;
use zeroize::Zeroize;

use rand::prelude::*;


#[cfg(feature = "nostd-stm32f4")]
fn get_rng() -> impl RngCore + CryptoRng {
    /* 
     * for now just looking for any sw that fills the gap,
     * if the HW contains a crypto quality HW RNG then this
     * might supply that
     * Normally uses an OsRng from getrandom that is not available
     * on some embedded platforms
     *
     * w/o another basic improvement is to gather a seed from some
     * other source (e.g. clock jitter, or A2D low order bits, etc)
     */
     let seed: [u8; 32] = [
        42, 3, 14, 19,
        1, 2, 3, 4,
        1, 2, 3, 4,
        1, 2, 3, 4,
        1, 2, 3, 4,
        1, 2, 3, 4,
        1, 2, 3, 4,
        1, 2, 3, 4
      ];
    return rand_chacha::ChaCha20Rng::from_seed(seed);
}

#[cfg(not(feature = "nostd-stm32f4"))]
fn get_rng() -> impl RngCore + CryptoRng {
    return rand::rngs::OsRng;
}


/// A pure rust implementation of a vault.
/// Is not thread-safe i.e. if multiple threads
/// add values to the vault there may be collisions
/// This is mostly for testing purposes anyway
/// and shouldn't be used for production
///
/// ```
/// use ockam_vault::software::DefaultVault;
/// let vault = DefaultVault::default();
/// ```
#[derive(Debug)]
pub struct DefaultVault {
    entries: VaultMap,
    next_id: usize,
}

impl Default for DefaultVault {
    fn default() -> Self {
        Self {
            entries: VaultMap::new(),
            next_id: 0,
        }
    }
}

impl DefaultVault {
    fn get_entry(
        &self,
        context: SecretKeyContext,
        error: VaultFailErrorKind,
    ) -> Result<&VaultEntry, VaultFailError> {
        let id;
        if let SecretKeyContext::Memory(i) = context {
            id = i;
        } else {
            fail!(error);
        }
        let entry;
        if let Some(e) = self.entries.get(&id) {
            entry = e;
        } else {
            fail!(error);
        }
        Ok(entry)
    }

    fn hkdf_sha256_internal(
        &mut self,
        salt: SecretKeyContext,
        info: &[u8],
        ikm: &[u8],
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<SecretKeyContext>, VaultFailError> {
        let salt = self.get_entry(salt, VaultFailErrorKind::Ecdh)?;

        // FIXME: Doesn't work for secrets with size more than 32 bytes
        let okm_len = output_attributes.len() * 32;

        let okm = match &salt.key {
            SecretKey::Buffer(salt) => {
                let mut okm = vec![0u8; okm_len];
                let prk = hkdf::Hkdf::<Sha256>::new(Some(salt), ikm);
                prk.expand(info, okm.as_mut_slice())?;
                Ok(okm)
            }
            _ => Err(VaultFailError::from_msg(
                VaultFailErrorKind::HkdfSha256,
                "Unknown key type",
            )),
        }?;

        let mut secrets = Vec::<SecretKeyContext>::new();
        let mut index = 0;

        for attributes in output_attributes {
            let secret = match attributes.xtype {
                SecretKeyType::Buffer(size) => {
                    Ok(SecretKey::Buffer(okm[index..index + size].to_vec()))
                }
                SecretKeyType::Aes256 => Ok(SecretKey::Aes256(*array_ref!(okm, index, 32))),
                SecretKeyType::Aes128 => Ok(SecretKey::Aes128(*array_ref!(okm, index, 16))),
                _ => Err(VaultFailError::from_msg(
                    VaultFailErrorKind::HkdfSha256,
                    "Unknown key type",
                )),
            }?;
            let secret = self.secret_import(&secret, attributes)?;

            secrets.push(secret);
            index += 32;
        }

        Ok(secrets)
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

#[derive(Debug, Eq, PartialEq, Zeroize)]
#[zeroize(drop)]
struct VaultEntry {
    id: usize,
    key_attributes: SecretKeyAttributes,
    key: SecretKey,
}

impl Default for VaultEntry {
    fn default() -> Self {
        Self {
            id: 0,
            key_attributes: SecretKeyAttributes {
                xtype: SecretKeyType::Curve25519,
                persistence: SecretPersistenceType::Ephemeral,
                purpose: SecretPurposeType::KeyAgreement,
            },
            key: SecretKey::Curve25519([0u8; 32]),
        }
    }
}

macro_rules! encrypt_op_impl {
    ($a:expr,$aad:expr,$nonce:expr,$text:expr,$type:ident,$op:ident) => {{
        let key = GenericArray::from_slice($a.as_ref());
        let cipher = $type::new(key);
        let nonce = GenericArray::from_slice($nonce.as_ref());
        let payload = Payload {
            aad: $aad.as_ref(),
            msg: $text.as_ref(),
        };
        let output = cipher.$op(nonce, payload)?;
        Ok(output)
    }};
}

macro_rules! encrypt_impl {
    ($entry:expr, $aad:expr, $nonce: expr, $text:expr, $op:ident, $err:expr) => {{
        match $entry.key {
            SecretKey::Aes128(a) => encrypt_op_impl!(a, $aad, $nonce, $text, Aes128Gcm, $op),
            SecretKey::Aes256(a) => encrypt_op_impl!(a, $aad, $nonce, $text, Aes256Gcm, $op),
            _ => Err($err.into()),
        }
    }};
}

impl Vault for DefaultVault {
    fn random(&mut self, data: &mut [u8]) -> Result<(), VaultFailError> {
        let mut rng = get_rng();
        rng.fill_bytes(data);
        Ok(())
    }

    fn sha256<B: AsRef<[u8]>>(&self, data: B) -> Result<[u8; 32], VaultFailError> {
        let digest = Sha256::digest(data.as_ref());
        Ok(*array_ref![digest, 0, 32])
    }

    fn secret_generate(
        &mut self,
        attributes: SecretKeyAttributes,
    ) -> Result<SecretKeyContext, VaultFailError> {
        let mut rng = get_rng();
        let key = match attributes.xtype {
            SecretKeyType::Curve25519 => {
                let sk = x25519_dalek::StaticSecret::new(&mut rng);
                SecretKey::Curve25519(sk.to_bytes())
            }
            SecretKeyType::Aes128 => {
                let mut key = [0u8; 16];
                rng.fill_bytes(&mut key);
                SecretKey::Aes128(key)
            }
            SecretKeyType::Aes256 => {
                let mut key = [0u8; 32];
                rng.fill_bytes(&mut key);
                SecretKey::Aes256(key)
            }
            SecretKeyType::P256 => {
                let key = p256::SecretKey::random(&mut rng);
                let mut value = [0u8; 32];
                value.copy_from_slice(&key.secret_scalar().to_bytes());
                SecretKey::P256(value)
            }
            SecretKeyType::Buffer(size) => {
                let mut key = vec![0u8; size];
                rng.fill_bytes(key.as_mut_slice());
                SecretKey::Buffer(key)
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
        Ok(SecretKeyContext::Memory(self.next_id))
    }

    fn secret_import(
        &mut self,
        secret: &SecretKey,
        attributes: SecretKeyAttributes,
    ) -> Result<SecretKeyContext, VaultFailError> {
        self.next_id += 1;
        self.entries.insert(
            self.next_id,
            VaultEntry {
                id: self.next_id,
                key_attributes: attributes,
                key: secret.clone(),
            },
        );
        Ok(SecretKeyContext::Memory(self.next_id))
    }

    fn secret_export(&mut self, context: SecretKeyContext) -> Result<SecretKey, VaultFailError> {
        if let SecretKeyContext::Memory(id) = context {
            self.entries
                .get(&id)
                .map(|i| i.key.clone())
                .ok_or_else(|| VaultFailErrorKind::GetAttributes.into())
        } else {
            Err(VaultFailErrorKind::Export.into())
        }
    }

    fn secret_attributes_get(
        &mut self,
        context: SecretKeyContext,
    ) -> Result<SecretKeyAttributes, VaultFailError> {
        if let SecretKeyContext::Memory(id) = context {
            self.entries
                .get(&id)
                .map(|i| i.key_attributes)
                .ok_or_else(|| VaultFailErrorKind::GetAttributes.into())
        } else {
            Err(VaultFailErrorKind::GetAttributes.into())
        }
    }

    fn secret_public_key_get(
        &mut self,
        context: SecretKeyContext,
    ) -> Result<PublicKey, VaultFailError> {
        let entry = self.get_entry(context, VaultFailErrorKind::PublicKey)?;

        match entry.key {
            SecretKey::Curve25519(a) => {
                let sk = x25519_dalek::StaticSecret::from(a);
                let pk = x25519_dalek::PublicKey::from(&sk);
                Ok(PublicKey::Curve25519(*pk.as_bytes()))
            }
            SecretKey::P256(a) => {
                let sk = Scalar::from_bytes_reduced(p256::FieldBytes::from_slice(&a));
                let pp = ProjectivePoint::generator() * sk;
                let ap = p256::elliptic_curve::sec1::EncodedPoint::from(pp.to_affine());
                Ok(PublicKey::P256(*array_ref![ap.as_bytes(), 0, 65]))
            }
            _ => Err(VaultFailErrorKind::PublicKey.into()),
        }
    }

    fn secret_destroy(&mut self, context: SecretKeyContext) -> Result<(), VaultFailError> {
        if let SecretKeyContext::Memory(id) = context {
            if let Some(mut k) = self.entries.remove(&id) {
                k.key.zeroize();
            }
            Ok(())
        } else {
            Err(VaultFailErrorKind::InvalidParam(0).into())
        }
    }

    fn ec_diffie_hellman(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
    ) -> Result<SecretKeyContext, VaultFailError> {
        let entry = self.get_entry(context, VaultFailErrorKind::Ecdh)?;

        let value = match (&entry.key, peer_public_key) {
            (SecretKey::Curve25519(a), PublicKey::Curve25519(b)) => {
                let sk = x25519_dalek::StaticSecret::from(*a);
                let pk_t = x25519_dalek::PublicKey::from(b);
                let secret = sk.diffie_hellman(&pk_t);
                Ok(secret.as_bytes().to_vec())
            }
            (SecretKey::P256(a), PublicKey::P256(b)) => {
                let o_pk_t = p256::elliptic_curve::sec1::EncodedPoint::from_bytes(b.as_ref());
                if o_pk_t.is_err() {
                    fail!(VaultFailErrorKind::Ecdh);
                }
                let pk_t = o_pk_t.unwrap();
                let o_p_t = AffinePoint::from_encoded_point(&pk_t);
                if o_p_t.is_none().unwrap_u8() == 1 {
                    fail!(VaultFailErrorKind::Ecdh);
                }
                let sk = Scalar::from_bytes_reduced(p256::FieldBytes::from_slice(a.as_ref()));
                let pk_t = ProjectivePoint::from(o_p_t.unwrap());
                let secret = pk_t * sk;
                if secret.is_identity().unwrap_u8() == 1 {
                    fail!(VaultFailErrorKind::Ecdh);
                }
                let ap = p256::elliptic_curve::sec1::EncodedPoint::from(secret.to_affine());
                Ok(ap.x().as_slice().to_vec())
            }
            (_, _) => Err(VaultFailError::from_msg(
                VaultFailErrorKind::Ecdh,
                "Unknown key type",
            )),
        }?;
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Buffer(value.len()),
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Ephemeral,
        };
        let secret = SecretKey::Buffer(value);
        self.secret_import(&secret, attributes)
    }

    fn ec_diffie_hellman_hkdf_sha256(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
        salt: SecretKeyContext,
        info: &[u8],
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<SecretKeyContext>, VaultFailError> {
        let private_key_entry = self.get_entry(context, VaultFailErrorKind::Ecdh)?;

        let dh = match (&private_key_entry.key, peer_public_key) {
            (SecretKey::Curve25519(a), PublicKey::Curve25519(b)) => {
                let sk = x25519_dalek::StaticSecret::from(*a);
                let pk_t = x25519_dalek::PublicKey::from(b);
                let secret = sk.diffie_hellman(&pk_t);
                Ok(secret.as_bytes().to_vec())
            }
            (SecretKey::P256(a), PublicKey::P256(b)) => {
                let o_pk_t = p256::elliptic_curve::sec1::EncodedPoint::from_bytes(b.as_ref());
                if o_pk_t.is_err() {
                    fail!(VaultFailErrorKind::Ecdh);
                }
                let pk_t = o_pk_t.unwrap();
                let o_p_t = AffinePoint::from_encoded_point(&pk_t);
                if o_p_t.is_none().unwrap_u8() == 1 {
                    fail!(VaultFailErrorKind::Ecdh);
                }
                let sk = Scalar::from_bytes_reduced(p256::FieldBytes::from_slice(a.as_ref()));
                let pk_t = ProjectivePoint::from(o_p_t.unwrap());
                let secret = pk_t * sk;
                if secret.is_identity().unwrap_u8() == 1 {
                    fail!(VaultFailErrorKind::Ecdh);
                }
                let ap = p256::elliptic_curve::sec1::EncodedPoint::from(secret.to_affine());
                Ok(ap.x().as_slice().to_vec())
            }
            (_, _) => Err(VaultFailError::from_msg(
                VaultFailErrorKind::Ecdh,
                "Unknown key type",
            )),
        }?;

        self.hkdf_sha256_internal(salt, info, &dh, output_attributes)
    }

    fn hkdf_sha256(
        &mut self,
        salt: SecretKeyContext,
        info: &[u8],
        ikm: Option<SecretKeyContext>,
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<SecretKeyContext>, VaultFailError> {
        let ikm_slice = match ikm {
            Some(ikm) => {
                let ikm = self.get_entry(ikm, VaultFailErrorKind::HkdfSha256)?;
                match &ikm.key {
                    SecretKey::Buffer(mem) => Ok(mem.as_slice().to_owned()),
                    _ => Err(VaultFailError::from_msg(
                        VaultFailErrorKind::HkdfSha256,
                        "Unknown key type",
                    )),
                }
            }
            None => Ok(Vec::new()),
        }?;

        self.hkdf_sha256_internal(salt, info, &ikm_slice, output_attributes)
    }

    fn aead_aes_gcm_encrypt<B: AsRef<[u8]>, C: AsRef<[u8]>, D: AsRef<[u8]>>(
        &mut self,
        context: SecretKeyContext,
        plaintext: B,
        nonce: C,
        aad: D,
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

    fn aead_aes_gcm_decrypt<B: AsRef<[u8]>, C: AsRef<[u8]>, D: AsRef<[u8]>>(
        &mut self,
        context: SecretKeyContext,
        cipher_text: B,
        nonce: C,
        aad: D,
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

    fn deinit(&mut self) {
        self.zeroize();
    }

    fn sign<B: AsRef<[u8]>>(
        &mut self,
        secret_key: SecretKeyContext,
        data: B,
    ) -> Result<[u8; 64], VaultFailError> {
        let entry = self.get_entry(secret_key, VaultFailErrorKind::Ecdh)?;
        match entry.key {
            SecretKey::Curve25519(k) => {
                let mut rng = get_rng();
                let mut nonce = [0u8; 64];
                rng.fill_bytes(&mut nonce);
                let sig = x25519_dalek::StaticSecret::from(k).sign(data.as_ref(), &nonce);
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

    fn verify<B: AsRef<[u8]>>(
        &mut self,
        signature: [u8; 64],
        public_key: PublicKey,
        data: B,
    ) -> Result<(), VaultFailError> {
        match public_key {
            PublicKey::Curve25519(k) => {
                if x25519_dalek::PublicKey::from(k).verify(data.as_ref(), &signature) {
                    Ok(())
                } else {
                    Err(VaultFailErrorKind::PublicKey.into())
                }
            }
            // PublicKey::P256(k) => {
            //     let key = VerifyKey::new(&k)?;
            //     let sig = Signature::try_from(&signature[..])?;
            //     key.verify(data.as_ref(), &sig)?;
            //     Ok(())
            // }
            _ => Err(VaultFailErrorKind::PublicKey.into()),
        }
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
        let mut attributes = SecretKeyAttributes {
            xtype: SecretKeyType::P256,
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };

        let res = vault.secret_generate(attributes);
        assert!(res.is_ok());
        let p256_ctx_1 = res.unwrap();

        let res = vault.secret_public_key_get(p256_ctx_1);
        assert!(res.is_ok());
        let pk_1 = res.unwrap();
        assert!(pk_1.is_p256());
        assert_eq!(vault.entries.len(), 1);
        assert_eq!(vault.next_id, 1);

        attributes.xtype = SecretKeyType::Curve25519;

        let res = vault.secret_generate(attributes);
        assert!(res.is_ok());
        let c25519_ctx_1 = res.unwrap();
        let res = vault.secret_public_key_get(c25519_ctx_1);
        assert!(res.is_ok());
        let pk_1 = res.unwrap();
        assert!(pk_1.is_curve25519());
        assert_eq!(vault.entries.len(), 2);
        assert_eq!(vault.next_id, 2);
    }

    #[test]
    fn new_secret_keys() {
        let mut vault = DefaultVault::default();
        let mut attributes = SecretKeyAttributes {
            xtype: SecretKeyType::P256,
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };
        let types = [
            (SecretKeyType::Curve25519, 32),
            (SecretKeyType::P256, 32),
            (SecretKeyType::Aes256, 32),
            (SecretKeyType::Aes128, 16),
            (SecretKeyType::Buffer(24), 24),
        ];
        for (t, s) in &types {
            attributes.xtype = *t;
            let res = vault.secret_generate(attributes);
            assert!(res.is_ok());
            let sk_ctx = res.unwrap();
            let sk = vault.secret_export(sk_ctx).unwrap();
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
        let salt = SecretKey::Buffer(salt_value.to_vec());
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Buffer(salt_value.len()),
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };
        let salt = vault.secret_import(&salt, attributes).unwrap();

        let ikm_value = b"a";
        let ikm = SecretKey::Buffer(ikm_value.to_vec());
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Buffer(ikm_value.len()),
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };
        let ikm = vault.secret_import(&ikm, attributes).unwrap();

        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Buffer(24),
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };

        let res = vault.hkdf_sha256(salt, b"", Some(ikm), vec![attributes]);
        assert!(res.is_ok());
        let digest = res.unwrap();
        assert_eq!(digest.len(), 1);
        let digest = vault.secret_export(digest[0]).unwrap();
        if let SecretKey::Buffer(digest) = &digest {
            assert_eq!(
                hex::encode(digest),
                "921ab9f260544b71941dbac2ca2d42c417aa07b53e055a8f"
            );
        } else {
            panic!();
        }
    }

    #[test]
    fn ec_diffie_hellman_p256() {
        let mut vault = DefaultVault::default();
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::P256,
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };
        let sk_ctx_1 = vault.secret_generate(attributes).unwrap();
        let sk_ctx_2 = vault.secret_generate(attributes).unwrap();
        let pk_1 = vault.secret_public_key_get(sk_ctx_1).unwrap();
        let pk_2 = vault.secret_public_key_get(sk_ctx_2).unwrap();
        let salt_value = b"ec_diffie_hellman_p256";
        let salt = SecretKey::Buffer(salt_value.to_vec());
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Buffer(salt_value.len()),
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };
        let salt = vault.secret_import(&salt, attributes).unwrap();

        let output_attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Buffer(32),
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };
        let res = vault.ec_diffie_hellman_hkdf_sha256(
            sk_ctx_1,
            pk_2,
            salt,
            b"",
            vec![output_attributes.clone()],
        );
        assert!(res.is_ok());
        let ss = res.unwrap();
        assert_eq!(ss.len(), 1);
        let ss = ss[0];
        let ss = vault.secret_export(ss).unwrap();
        if let SecretKey::Buffer(ss) = &ss {
            assert_eq!(ss.len(), 32);
        // FIXME: Check actual value
        } else {
            panic!();
        }

        let res = vault.ec_diffie_hellman_hkdf_sha256(
            sk_ctx_2,
            pk_1,
            salt,
            b"",
            vec![output_attributes.clone()],
        );
        assert!(res.is_ok());
        let ss = res.unwrap();
        assert_eq!(ss.len(), 1);
        let ss = ss[0];
        let ss = vault.secret_export(ss).unwrap();
        if let SecretKey::Buffer(ss) = &ss {
            assert_eq!(ss.len(), 32);
        // FIXME: Check actual value
        } else {
            panic!();
        }
        let res = vault.ec_diffie_hellman_hkdf_sha256(
            sk_ctx_1,
            pk_1,
            salt,
            b"",
            vec![output_attributes.clone()],
        );
        assert!(res.is_ok());
        let ss = res.unwrap();
        assert_eq!(ss.len(), 1);
        let ss = ss[0];
        let ss = vault.secret_export(ss).unwrap();
        if let SecretKey::Buffer(ss) = &ss {
            assert_eq!(ss.len(), 32);
        // FIXME: Check actual value
        } else {
            panic!();
        }
        let res = vault.ec_diffie_hellman_hkdf_sha256(
            sk_ctx_2,
            pk_2,
            salt,
            b"",
            vec![output_attributes.clone()],
        );
        assert!(res.is_ok());
        let ss = res.unwrap();
        assert_eq!(ss.len(), 1);
        let ss = ss[0];
        let ss = vault.secret_export(ss).unwrap();
        if let SecretKey::Buffer(ss) = &ss {
            assert_eq!(ss.len(), 32);
        // FIXME: Check actual value
        } else {
            panic!();
        }
    }

    #[test]
    fn ec_diffie_hellman_curve25519() {
        let mut vault = DefaultVault::default();
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Curve25519,
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };
        let sk_ctx_1 = vault.secret_generate(attributes).unwrap();
        let sk_ctx_2 = vault.secret_generate(attributes).unwrap();
        let pk_1 = vault.secret_public_key_get(sk_ctx_1).unwrap();
        let pk_2 = vault.secret_public_key_get(sk_ctx_2).unwrap();
        let salt_value = b"ec_diffie_hellman_curve25519";
        let salt = SecretKey::Buffer(salt_value.to_vec());
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Buffer(salt_value.len()),
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };
        let salt = vault.secret_import(&salt, attributes).unwrap();
        let output_attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Buffer(32),
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };
        let res = vault.ec_diffie_hellman_hkdf_sha256(
            sk_ctx_1,
            pk_2,
            salt,
            b"",
            vec![output_attributes.clone()],
        );
        assert!(res.is_ok());
        let ss = res.unwrap();
        assert_eq!(ss.len(), 1);
        let ss = ss[0];
        let ss = vault.secret_export(ss).unwrap();
        if let SecretKey::Buffer(ss) = &ss {
            assert_eq!(ss.len(), 32);
        // FIXME: Check actual value
        } else {
            panic!();
        }
        let res = vault.ec_diffie_hellman_hkdf_sha256(
            sk_ctx_2,
            pk_1,
            salt,
            b"",
            vec![output_attributes.clone()],
        );
        assert!(res.is_ok());
        let ss = res.unwrap();
        assert_eq!(ss.len(), 1);
        let ss = ss[0];
        let ss = vault.secret_export(ss).unwrap();
        if let SecretKey::Buffer(ss) = &ss {
            assert_eq!(ss.len(), 32);
        // FIXME: Check actual value
        } else {
            panic!();
        }
        let res = vault.ec_diffie_hellman_hkdf_sha256(
            sk_ctx_1,
            pk_1,
            salt,
            b"",
            vec![output_attributes.clone()],
        );
        assert!(res.is_ok());
        let ss = res.unwrap();
        assert_eq!(ss.len(), 1);
        let ss = ss[0];
        let ss = vault.secret_export(ss).unwrap();
        if let SecretKey::Buffer(ss) = &ss {
            assert_eq!(ss.len(), 32);
        // FIXME: Check actual value
        } else {
            panic!();
        }

        let res = vault.ec_diffie_hellman_hkdf_sha256(
            sk_ctx_2,
            pk_2,
            salt,
            b"",
            vec![output_attributes.clone()],
        );
        assert!(res.is_ok());
        let ss = res.unwrap();
        assert_eq!(ss.len(), 1);
        let ss = ss[0];
        let ss = vault.secret_export(ss).unwrap();
        if let SecretKey::Buffer(ss) = &ss {
            assert_eq!(ss.len(), 32);
        // FIXME: Check actual value
        } else {
            panic!();
        }
    }

    #[test]
    fn ec_diffie_hellman_different_keys() {
        let mut vault = DefaultVault::default();
        let mut attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Curve25519,
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };
        let sk_ctx_1 = vault.secret_generate(attributes).unwrap();
        let pk_1 = vault.secret_public_key_get(sk_ctx_1).unwrap();
        attributes.xtype = SecretKeyType::P256;
        let sk_ctx_2 = vault.secret_generate(attributes).unwrap();
        let pk_2 = vault.secret_public_key_get(sk_ctx_2).unwrap();

        let res = vault.ec_diffie_hellman(sk_ctx_1, pk_2);
        assert!(res.is_err());
        let res = vault.ec_diffie_hellman(sk_ctx_2, pk_1);
        assert!(res.is_err());
    }

    #[test]
    fn encryption() {
        let mut vault = DefaultVault::default();
        let message = b"Ockam Test Message";
        let nonce = b"TestingNonce";
        let aad = b"Extra payload data";
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Aes128,
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };

        let ctx = vault.secret_generate(attributes).unwrap();
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
            .secret_generate(SecretKeyAttributes {
                persistence: SecretPersistenceType::Ephemeral,
                purpose: SecretPurposeType::KeyAgreement,
                xtype: SecretKeyType::Curve25519,
            })
            .unwrap();
        let res = vault.sign(secret, b"hello world!");
        assert!(res.is_ok());
        let pubkey = vault.secret_public_key_get(secret).unwrap();
        let signature = res.unwrap();
        let res = vault.verify(signature, pubkey, b"hello world!");
        assert!(res.is_ok());
    }
}
