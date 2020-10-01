use std::path::PathBuf;

use ockam_vault::{error::*, software::DefaultVault, types::*, Vault};

use zeroize::Zeroize;

pub struct FilesystemVault {
    v: DefaultVault,
    path: PathBuf,
}

impl FilesystemVault {
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        let create_path = path.clone();
        std::fs::create_dir_all(create_path)?;

        let mut vault = DefaultVault::default();
        let to_secret = |path: PathBuf,
                         data: &[u8]|
         -> Result<(SecretKey, SecretKeyAttributes), VaultFailError> {
            let attrs = SecretKeyAttributes {
                xtype: SecretKeyType::Buffer(data.len()),
                persistence: SecretPersistenceType::Persistent,
                purpose: SecretPurposeType::KeyAgreement,
            };
            let mut bytes = attrs.to_bytes().to_vec();
            bytes.extend_from_slice(data);

            Ok((SecretKey::new(bytes, attrs.xtype), attrs))
        };

        let secret_path = path.clone();
        let fs_path = path.clone();

        path.clone().read_dir()?.for_each(|entry| {
            if let Ok(entry) = entry {
                match std::fs::read(entry.path()) {
                    Ok(data) => {
                        let (secret, attrs) =
                            &to_secret(secret_path.clone(), data.as_slice()).unwrap();
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

impl Vault for FilesystemVault {
    /// Generate random bytes and fill them into `data`
    fn random(&mut self, data: &mut [u8]) -> Result<(), VaultFailError> {
        self.v.random(data)
    }

    /// Compute the SHA-256 digest given input `data`
    fn sha256<B: AsRef<[u8]>>(&self, data: B) -> Result<[u8; 32], VaultFailError> {
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
        match ctx {
            SecretKeyContext::Memory(id) => {
                std::fs::write(self.path.join(id_to_path(id)), secret)?;
            }
            _ => {}
        }

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
        match ctx {
            SecretKeyContext::Memory(id) => {
                std::fs::write(self.path.join(id_to_path(id)), secret)?;
            }
            _ => {}
        }

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
        match context {
            SecretKeyContext::Memory(id) => {
                std::fs::remove_file(self.path.join(id_to_path(id)))
                    .map_err(|_| VaultFailErrorKind::IOError)?;
            }
            _ => {}
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
    fn ec_diffie_hellman_hkdf_sha256<B: AsRef<[u8]>>(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
        salt: B,
        okm_len: usize,
    ) -> Result<Vec<u8>, VaultFailError> {
        self.v
            .ec_diffie_hellman_hkdf_sha256(context, peer_public_key, salt, okm_len)
    }

    /// Compute the HKDF-SHA256 using the specified salt and input key material
    ///
    /// and return the output key material of the specified length
    fn hkdf_sha256<B: AsRef<[u8]>, C: AsRef<[u8]>>(
        &mut self,
        salt: B,
        ikm: C,
        okm_len: usize,
    ) -> Result<Vec<u8>, VaultFailError> {
        self.v.hkdf_sha256(salt, ikm, okm_len)
    }

    /// Encrypt a payload using AES-GCM
    fn aead_aes_gcm_encrypt<B: AsRef<[u8]>, C: AsRef<[u8]>, D: AsRef<[u8]>>(
        &mut self,
        context: SecretKeyContext,
        plaintext: B,
        nonce: C,
        aad: D,
    ) -> Result<Vec<u8>, VaultFailError> {
        self.v.aead_aes_gcm_encrypt(context, plaintext, nonce, aad)
    }

    /// Decrypt a payload using AES-GCM
    fn aead_aes_gcm_decrypt<B: AsRef<[u8]>, C: AsRef<[u8]>, D: AsRef<[u8]>>(
        &mut self,
        context: SecretKeyContext,
        cipher_text: B,
        nonce: C,
        aad: D,
    ) -> Result<Vec<u8>, VaultFailError> {
        self.v
            .aead_aes_gcm_decrypt(context, cipher_text, nonce, aad)
    }

    /// Close and release all resources in use by the vault
    fn deinit(&mut self) {
        self.v.deinit()
    }
}

impl Zeroize for FilesystemVault {
    fn zeroize(&mut self) {
        self.v.zeroize();
    }
}
