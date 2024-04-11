use colorful::Colorful;
use indoc::formatdoc;

use ockam_api::cli_state::vaults::NamedVault;
use ockam_api::colors::OckamColor;
use ockam_api::output::Output;

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
    fn single(&self) -> ockam_api::Result<String> {
        Ok(formatdoc!(
            r#"
            Vault:
                Name: {name}
                Type: {vault_type}
                Path: {vault_path}
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
            vault_path = self
                .vault
                .path_as_string()
                .color(OckamColor::PrimaryResource.color()),
        ))
    }

    fn list(&self) -> ockam_api::Result<String> {
        Ok(formatdoc!(
            r#"Name: {name}
            Type: {vault_type}
            Path: {vault_path}"#,
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
            vault_path = self
                .vault
                .path_as_string()
                .color(OckamColor::PrimaryResource.color()),
        ))
    }
}
