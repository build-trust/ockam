use crate::error::Error;
use crate::software_vault::{SoftwareVault, VaultEntry};
use arrayref::array_ref;
use ockam_vault_core::hash_vault::HashVault;
use ockam_vault_core::secret::Secret;
use ockam_vault_core::secret_vault::SecretVault;
use ockam_vault_core::types::{
    PublicKey, SecretAttributes, SecretKey, SecretType, CURVE25519_SECRET_LENGTH,
};
use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::Zeroize;

impl SecretVault for SoftwareVault {
    fn secret_generate(
        &mut self,
        attributes: SecretAttributes,
    ) -> Result<Secret, ockam_core::Error> {
        let mut rng = OsRng {};
        let (key, kid) = match attributes.stype {
            SecretType::Curve25519 => {
                let sk = x25519_dalek::StaticSecret::new(&mut rng);
                let public = x25519_dalek::PublicKey::from(&sk);
                let private = SecretKey::new(sk.to_bytes().to_vec());
                let kid = self.sha256(public.as_bytes())?;

                // FIXME: kid computation should be in one place
                (private, Some(hex::encode(kid)))
            }
            SecretType::Buffer => {
                let mut key = vec![0u8; attributes.length];
                rng.fill_bytes(key.as_mut_slice());
                (SecretKey::new(key), None)
            }
            _ => unimplemented!(),
        };
        self.next_id += 1;
        self.entries
            .insert(self.next_id, VaultEntry::new(kid, attributes, key));

        Ok(Secret::new(self.next_id))
    }

    fn secret_import(
        &mut self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> Result<Secret, ockam_core::Error> {
        // FIXME: Should we check secrets here?
        self.next_id += 1;
        self.entries.insert(
            self.next_id,
            VaultEntry::new(
                /* FIXME */ None,
                attributes,
                SecretKey::new(secret.to_vec()),
            ),
        );
        Ok(Secret::new(self.next_id))
    }

    fn secret_export(&mut self, context: &Secret) -> Result<SecretKey, ockam_core::Error> {
        self.get_entry(context).map(|i| i.key().clone())
    }

    fn secret_attributes_get(
        &mut self,
        context: &Secret,
    ) -> Result<SecretAttributes, ockam_core::Error> {
        self.get_entry(context).map(|i| i.key_attributes())
    }

    fn secret_public_key_get(&mut self, context: &Secret) -> Result<PublicKey, ockam_core::Error> {
        let entry = self.get_entry(context)?;

        if entry.key().as_ref().len() != CURVE25519_SECRET_LENGTH {
            return Err(Error::InvalidPrivateKeyLen.into());
        }

        match entry.key_attributes().stype {
            SecretType::Curve25519 => {
                let sk = x25519_dalek::StaticSecret::from(*array_ref![
                    entry.key().as_ref(),
                    0,
                    CURVE25519_SECRET_LENGTH
                ]);
                let pk = x25519_dalek::PublicKey::from(&sk);
                Ok(PublicKey::new(pk.to_bytes().to_vec()))
            }
            _ => Err(Error::InvalidKeyType.into()),
        }
    }

    fn secret_destroy(&mut self, context: Secret) -> Result<(), ockam_core::Error> {
        if let Some(mut k) = self.entries.remove(&context.index()) {
            k.zeroize();
        }
        Ok(())
    }
}
