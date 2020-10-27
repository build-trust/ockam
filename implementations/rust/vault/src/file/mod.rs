use std::convert::TryFrom;
use std::fs;
use std::path::PathBuf;

use crate::{error::*, software::DefaultVault, types::*, DynVault};

use zeroize::Zeroize;

const ATTRS_BYTE_LENGTH: usize = 6;

/// A Vault that persists keys to the file system in a specified directory.
/// Each key is in its own file
#[derive(Debug)]
pub struct FilesystemVault {
    v: DefaultVault,
    path: PathBuf,
}

impl FilesystemVault {
    /// Create a new FilesystemVault where keys are stored in `path`
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        let create_path = path.clone();
        fs::create_dir_all(create_path)?;

        let mut vault = DefaultVault::default();
        let to_secret = |data: &[u8]| -> Result<(SecretKey, SecretKeyAttributes), VaultFailError> {
            if data.len() < ATTRS_BYTE_LENGTH {
                return Err(VaultFailErrorKind::InvalidSecret.into());
            }

            let mut attrs = [0u8; ATTRS_BYTE_LENGTH];
            attrs.copy_from_slice(&data[0..ATTRS_BYTE_LENGTH]);
            let attributes = SecretKeyAttributes::try_from(attrs)?;

            Ok((
                SecretKey::new(&data[ATTRS_BYTE_LENGTH..], attributes.xtype),
                attributes,
            ))
        };
        let fs_path = path.clone();

        path.read_dir()?
            .filter(|r| {
                // ignore directories within vault path
                if let Ok(e) = r {
                    match fs::metadata(e.path()) {
                        Ok(md) => md.is_file(),
                        Err(_) => false,
                    }
                } else {
                    false
                }
            })
            .for_each(|entry| {
                if let Ok(entry) = entry {
                    match fs::read(entry.path()) {
                        Ok(data) => {
                            let (secret, attrs) =
                                &to_secret(data.as_slice()).unwrap_or_else(|_| {
                                    panic!("failed to get secret {:?} from file", entry.file_name())
                                });
                            if let Err(e) = vault.secret_import(secret, *attrs) {
                                eprintln!("{}", e);
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            });

        Ok(Self {
            v: vault,
            path: fs_path,
        })
    }
}

fn id_to_path(id: usize) -> PathBuf {
    id.to_string().into()
}

fn fs_write_secret(
    path: PathBuf,
    ctx: SecretKeyContext,
    key: SecretKey,
    attrs: SecretKeyAttributes,
) -> Result<(), VaultFailError> {
    match ctx {
        SecretKeyContext::Memory(id) => {
            let mut bytes = attrs.to_bytes().to_vec();
            bytes.extend_from_slice(key.as_ref());

            Ok(fs::write(path.join(id_to_path(id)), bytes)?)
        }
        _ => Err(VaultFailErrorKind::InvalidContext.into()),
    }
}

impl DynVault for FilesystemVault {
    /// Generate random bytes and fill them into `data`
    fn random(&mut self, data: &mut [u8]) -> Result<(), VaultFailError> {
        self.v.random(data)
    }

    /// Compute the SHA-256 digest given input `data`
    fn sha256(&self, data: &[u8]) -> Result<[u8; 32], VaultFailError> {
        self.v.sha256(data)
    }

    /// Create a new secret key
    fn secret_generate(
        &mut self,
        attributes: SecretKeyAttributes,
    ) -> Result<SecretKeyContext, VaultFailError> {
        // write the secret to disk using the context id
        let ctx = self.v.secret_generate(attributes)?;
        let secret = self.v.secret_export(ctx)?;
        fs_write_secret(self.path.clone(), ctx, secret, attributes)?;

        Ok(ctx)
    }

    /// Import a secret key into the vault
    fn secret_import(
        &mut self,
        secret: &SecretKey,
        attributes: SecretKeyAttributes,
    ) -> Result<SecretKeyContext, VaultFailError> {
        // write the secret to disk using the context id
        let ctx = self.v.secret_import(secret, attributes)?;
        fs_write_secret(self.path.clone(), ctx, secret.clone(), attributes)?;

        Ok(ctx)
    }

    /// Export a secret key from the vault
    fn secret_export(&mut self, context: SecretKeyContext) -> Result<SecretKey, VaultFailError> {
        self.v.secret_export(context)
    }

    /// Get the attributes for a secret key
    fn secret_attributes_get(
        &mut self,
        context: SecretKeyContext,
    ) -> Result<SecretKeyAttributes, VaultFailError> {
        self.v.secret_attributes_get(context)
    }

    /// Return the associated public key given the secret key
    fn secret_public_key_get(
        &mut self,
        context: SecretKeyContext,
    ) -> Result<PublicKey, VaultFailError> {
        self.v.secret_public_key_get(context)
    }

    /// Remove a secret key from the vault
    fn secret_destroy(&mut self, context: SecretKeyContext) -> Result<(), VaultFailError> {
        self.v.secret_destroy(context)?;

        if let SecretKeyContext::Memory(id) = context {
            let path = self.path.join(id_to_path(id));
            match fs::metadata(path.clone()) {
                Ok(md) if md.is_file() => {
                    fs::remove_file(path).map_err(|_| VaultFailErrorKind::IOError)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    ///
    /// and the specified uncompressed public key
    fn ec_diffie_hellman(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
    ) -> Result<SecretKeyContext, VaultFailError> {
        self.v.ec_diffie_hellman(context, peer_public_key)
    }

    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    ///
    /// and the specified uncompressed public key and return the HKDF-SHA256
    ///
    /// output using the DH value as the HKDF ikm
    fn ec_diffie_hellman_hkdf_sha256(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
        salt: SecretKeyContext,
        info: &[u8],
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<SecretKeyContext>, VaultFailError> {
        self.v.ec_diffie_hellman_hkdf_sha256(
            context,
            peer_public_key,
            salt,
            info,
            output_attributes,
        )
    }

    /// Compute the HKDF-SHA256 using the specified salt and input key material
    ///
    /// and return the output key material of the specified length
    fn hkdf_sha256(
        &mut self,
        salt: SecretKeyContext,
        info: &[u8],
        ikm: Option<SecretKeyContext>,
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<SecretKeyContext>, VaultFailError> {
        self.v.hkdf_sha256(salt, info, ikm, output_attributes)
    }

    /// Encrypt a payload using AES-GCM
    fn aead_aes_gcm_encrypt(
        &mut self,
        context: SecretKeyContext,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, VaultFailError> {
        self.v.aead_aes_gcm_encrypt(context, plaintext, nonce, aad)
    }

    /// Decrypt a payload using AES-GCM
    fn aead_aes_gcm_decrypt(
        &mut self,
        context: SecretKeyContext,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, VaultFailError> {
        self.v
            .aead_aes_gcm_decrypt(context, cipher_text, nonce, aad)
    }

    /// Close and release all resources in use by the vault
    fn deinit(&mut self) {
        self.v.deinit()
    }

    fn sign(
        &mut self,
        secret_key: SecretKeyContext,
        data: &[u8],
    ) -> Result<[u8; 64], VaultFailError> {
        self.v.sign(secret_key, data)
    }

    fn verify(
        &mut self,
        signature: [u8; 64],
        public_key: PublicKey,
        data: &[u8],
    ) -> Result<(), VaultFailError> {
        self.v.verify(signature, public_key, data)
    }
}

impl Zeroize for FilesystemVault {
    fn zeroize(&mut self) {
        self.v.zeroize();
    }
}
