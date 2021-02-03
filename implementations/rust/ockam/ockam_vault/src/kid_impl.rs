use crate::software_vault::SoftwareVault;
use crate::VaultError;
use ockam_vault_core::{HashVault, Kid, KidVault, PublicKey, Secret};

impl KidVault for SoftwareVault {
    fn get_secret_by_kid(&self, kid: &str) -> ockam_core::Result<Secret> {
        let index = self
            .entries
            .iter()
            .find(|(_, entry)| {
                if let Some(e_kid) = entry.kid() {
                    e_kid == kid
                } else {
                    false
                }
            })
            .ok_or(VaultError::SecretNotFound.into())?
            .0;

        Ok(Secret::new(*index))
    }

    fn compute_kid_for_public_key(&self, public_key: &PublicKey) -> ockam_core::Result<Kid> {
        let kid = self.sha256(public_key.as_ref())?;
        Ok(hex::encode(kid).into())
    }
}

#[cfg(test)]
mod tests {
    use crate::SoftwareVault;
    use ockam_vault_core::{
        KidVault, PublicKey, SecretAttributes, SecretPersistence, SecretType, SecretVault,
        CURVE25519_SECRET_LENGTH,
    };

    #[test]
    fn compute_kid_for_public_key() {
        let vault = SoftwareVault::new();

        let public =
            hex::decode("68858ea1ea4e1ade755df7fb6904056b291d9781eb5489932f46e32f12dd192a")
                .unwrap();
        let public = PublicKey::new(public.to_vec().into());

        let kid = vault.compute_kid_for_public_key(&public).unwrap();

        assert_eq!(
            kid,
            "732af49a0b47c820c0a4cac428d6cb80c1fa70622f4a51708163dd87931bc942"
        );
    }

    #[test]
    fn get_secret_by_kid() {
        let mut vault = SoftwareVault::new();

        let attributes = SecretAttributes {
            stype: SecretType::Curve25519,
            persistence: SecretPersistence::Ephemeral,
            length: CURVE25519_SECRET_LENGTH,
        };

        let secret = vault.secret_generate(attributes).unwrap();
        let public = vault.secret_public_key_get(&secret).unwrap();

        let kid = vault.compute_kid_for_public_key(&public).unwrap();
        let secret2 = vault.get_secret_by_kid(&kid).unwrap();

        assert_eq!(secret.index(), secret2.index());
    }
}
