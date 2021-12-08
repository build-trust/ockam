use crate::VaultError;
use ockam_core::compat::{string::ToString, vec::Vec};
use ockam_core::vault::{PublicKey, SecretType};
use ockam_core::Result;

/// Helper for importing OpenSSH keys
pub struct OpenSshKeys;

impl OpenSshKeys {
    /// Extract raw ed25519 private key from OpenSSH string representation
    pub fn extract_raw_ed25519_secret_key(key_str: &str) -> Result<Vec<u8>> {
        // TODO: Replace with proper parsing

        let lines: Vec<&str> = key_str.lines().collect();

        if lines.len() < 3 {
            return Err(VaultError::InvalidOpenSshSecret.into());
        }

        let first = lines.first().unwrap();
        let last = lines.last().unwrap();

        let mut key_str = "".to_string();
        for line in &lines[1..lines.len() - 1] {
            key_str.push_str(line);
        }

        if first != &"-----BEGIN OPENSSH PRIVATE KEY-----" {
            return Err(VaultError::InvalidOpenSshSecret.into());
        }

        if last != &"-----END OPENSSH PRIVATE KEY-----" {
            return Err(VaultError::InvalidOpenSshSecret.into());
        }

        let key_data = base64::decode(key_str).map_err(|_| VaultError::InvalidOpenSshSecret)?;

        if key_data.len() < 193 {
            return Err(VaultError::InvalidOpenSshSecret.into());
        }

        let key_data = key_data[161..193].to_vec();

        Ok(key_data)
    }

    /// Extract raw ed25519 public key from OpenSSH string representation
    pub fn extract_ed25519_public_key(key_str: &str) -> Result<PublicKey> {
        // TODO: Replace with proper parsing

        let mut split = key_str.split_whitespace();

        if let Some(kt) = split.next() {
            if kt != "ssh-ed25519" {
                return Err(VaultError::InvalidOpenSshPublicKey.into());
            }
        } else {
            return Err(VaultError::InvalidOpenSshPublicKey.into());
        }

        let key_str;
        if let Some(ks) = split.next() {
            key_str = ks;
        } else {
            return Err(VaultError::InvalidOpenSshPublicKey.into());
        }

        let key_data = base64::decode(key_str).map_err(|_| VaultError::InvalidOpenSshPublicKey)?;

        if key_data.len() != 51 {
            return Err(VaultError::InvalidOpenSshPublicKey.into());
        }

        let key_data = key_data[19..].to_vec();

        Ok(PublicKey::new(key_data, SecretType::Ed25519))
    }
}

#[cfg(test)]
mod tests {
    use crate::ockam_core::vault::{SecretPersistence, SecretType};
    use crate::OpenSshKeys;
    use crate::{SecretAttributes, SecretVault, Signer, SoftwareVault, Verifier};

    const VALID_SECRET_KEY: &'static str = "-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACD8wfh3Dam8lP1avwWXpFbLCZIuL3BlAgz+gYDKxiPERgAAAKB4l3KgeJdy
oAAAAAtzc2gtZWQyNTUxOQAAACD8wfh3Dam8lP1avwWXpFbLCZIuL3BlAgz+gYDKxiPERg
AAAECJ7gnmFRfhIuAYmL+TXjW8GTZ6G9DuRzk2IA4cCwz9r/zB+HcNqbyU/Vq/BZekVssJ
ki4vcGUCDP6BgMrGI8RGAAAAFnlvdXJfZW1haWxAZXhhbXBsZS5jb20BAgMEBQYH
-----END OPENSSH PRIVATE KEY-----";

    const VALID_PUBLIC_KEY: &'static str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPzB+HcNqbyU/Vq/BZekVssJki4vcGUCDP6BgMrGI8RG your_email@example.com";

    #[allow(non_snake_case)]
    #[tokio::test]
    async fn extract_keys__correct_key_pair__should_succeed() {
        let secret_key_data =
            OpenSshKeys::extract_raw_ed25519_secret_key(VALID_SECRET_KEY).unwrap();
        let public_key = OpenSshKeys::extract_ed25519_public_key(VALID_PUBLIC_KEY).unwrap();

        let mut vault = SoftwareVault::default();

        let secret = vault
            .secret_import(
                &secret_key_data,
                SecretAttributes::new(SecretType::Ed25519, SecretPersistence::Ephemeral, 32),
            )
            .await
            .unwrap();

        let msg = b"TEST";
        let signature = vault.sign(&secret, msg).await.unwrap();

        let res = vault.verify(&signature, &public_key, msg).await.unwrap();

        assert!(res)
    }
}
