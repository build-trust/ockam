use colorful::Colorful;
use indoc::formatdoc;

use ockam_api::cli_state::vaults::NamedVault;

use crate::output::Output;
use crate::OckamColor;

#[derive(serde::Serialize)]
pub struct VaultOutput {
    vault: NamedVault,
}

impl VaultOutput {
    pub fn new(vault: &NamedVault) -> Self {
        Self {
            vault: vault.clone(),
        }
    }

    pub fn name(&self) -> String {
        self.vault.name().clone()
    }
}

impl Output for VaultOutput {
    fn output(&self) -> crate::error::Result<String> {
        Ok(formatdoc!(
            r#"
            Vault:
                Name: {name}
                Type: {vault_type}
            "#,
            name = self
                .vault
                .name()
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            vault_type = match self.vault.is_kms() {
                true => "AWS KMS",
                false => "OCKAM",
            }
            .to_string()
            .color(OckamColor::PrimaryResource.color()),
        ))
    }

    fn list_output(&self) -> crate::error::Result<String> {
        Ok(formatdoc!(
            r#"Name: {name}
            Type: {vault_type}"#,
            name = self
                .vault
                .name()
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            vault_type = match self.vault.is_kms() {
                true => "AWS KMS",
                false => "OCKAM",
            }
            .to_string()
            .color(OckamColor::PrimaryResource.color()),
        ))
    }
}
