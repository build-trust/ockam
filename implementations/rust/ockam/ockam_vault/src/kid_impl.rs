use crate::error::Error;
use crate::software_vault::SoftwareVault;
use ockam_vault_core::{KidVault, Secret};

impl KidVault for SoftwareVault {
    fn get_secret_by_kid(&self, kid: &str) -> Result<Secret, ockam_core::Error> {
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
            .ok_or(Error::SecretNotFound.into())?
            .0;

        Ok(Secret::new(*index))
    }
}
