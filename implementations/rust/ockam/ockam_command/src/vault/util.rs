use colorful::Colorful;
use indoc::formatdoc;

use ockam_api::cli_state::vaults::NamedVault;
use ockam_api::cli_state::{UseAwsKms, VaultType};
use ockam_api::colors::OckamColor;
use ockam_api::output::{indent, Output};

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
    fn item(&self) -> ockam_api::Result<String> {
        Ok(formatdoc!(
            r#"
            Vault:
            {vault}
            "#,
            vault = indent("   ", self.as_list_item()?)
        ))
    }

    fn as_list_item(&self) -> ockam_api::Result<String> {
        let name = self
            .vault
            .name()
            .to_string()
            .color(OckamColor::PrimaryResource.color());

        let vault_type = match self.vault.vault_type() {
            VaultType::DatabaseVault { .. } => "Internal",
            VaultType::LocalFileVault { .. } => "External",
            VaultType::RemoteVault { .. } => "Remote",
        }
        .to_string()
        .color(OckamColor::PrimaryResource.color());

        let uses_aws_kms = if self.vault.use_aws_kms() {
            "true"
        } else {
            "false"
        }
        .to_string()
        .color(OckamColor::PrimaryResource.color());

        Ok(match self.vault.vault_type() {
            VaultType::DatabaseVault {
                use_aws_kms: UseAwsKms::No,
            } => formatdoc!(
                r#"Name: {name}
                   Type: {vault_type}"#,
                name = name,
                vault_type = vault_type
            ),
            VaultType::DatabaseVault {
                use_aws_kms: UseAwsKms::Yes,
            } => formatdoc!(
                r#"Name: {name}
            Type: {vault_type}
            Uses AWS KMS: {uses_aws_kms}"#,
                name = name,
                uses_aws_kms = uses_aws_kms
            ),
            VaultType::LocalFileVault {
                path,
                use_aws_kms: UseAwsKms::No,
            } => formatdoc!(
                r#"Name: {name}
            Type: {vault_type}
            Path: {vault_path}"#,
                name = name,
                vault_path = path
                    .to_string_lossy()
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ),
            VaultType::LocalFileVault {
                path,
                use_aws_kms: UseAwsKms::Yes,
            } => formatdoc!(
                r#"Name: {name}
            Type: {vault_type}
            Path: {vault_path}
            Uses AWS KMS: {uses_aws_kms}"#,
                name = name,
                vault_path = path
                    .to_string_lossy()
                    .to_string()
                    .color(OckamColor::PrimaryResource.color()),
                uses_aws_kms = uses_aws_kms,
            ),
            VaultType::RemoteVault {
                vault_multiaddr,
                local_identifier,
                authority_identifier,
                authority_multiaddr,
                credential_scope,
            } => formatdoc!(
                r#"Name: {name}
            Type: {vault_type}
            Route: {route}
            Local Identity: {local_identifier}
            Authority Identifier: {authority_identifier}
            Authority Route: {authority_multiaddr}
            Credential Scope: {credential_scope}"#,
                route = vault_multiaddr
                    .to_string()
                    .color(OckamColor::PrimaryResource.color()),
                local_identifier = local_identifier
                    .to_string()
                    .color(OckamColor::PrimaryResource.color()),
                authority_identifier = authority_identifier
                    .to_string()
                    .color(OckamColor::PrimaryResource.color()),
                authority_multiaddr = authority_multiaddr
                    .to_string()
                    .color(OckamColor::PrimaryResource.color()),
                credential_scope = credential_scope
                    .to_string()
                    .color(OckamColor::PrimaryResource.color()),
            ),
        })
    }
}
