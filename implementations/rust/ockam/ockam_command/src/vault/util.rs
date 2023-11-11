use crate::output::Output;
use crate::OckamColor;
use colorful::Colorful;
use indoc::formatdoc;
use ockam_api::cli_state::{StateItemTrait, VaultConfig, VaultState};

#[derive(serde::Serialize)]
pub struct VaultOutput {
    pub(crate) name: String,
    #[serde(flatten)]
    config: VaultConfig,
    is_default: bool,
}

impl VaultOutput {
    pub fn new(state: &VaultState, is_default: bool) -> Self {
        Self {
            name: state.name().to_string(),
            config: state.config().clone(),
            is_default,
        }
    }
}

impl Output for VaultOutput {
    fn output(&self) -> crate::error::Result<String> {
        Ok(formatdoc!(
            r#"
            Vault:
                Name: {name} {default}
                Type: {vault_type}
            "#,
            name = self
                .name
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            default = if self.is_default { "(default)" } else { "" },
            vault_type = match self.config.is_aws() {
                true => "AWS KMS",
                false => "OCKAM",
            }
            .to_string()
            .color(OckamColor::PrimaryResource.color()),
        ))
    }

    fn list_output(&self) -> crate::error::Result<String> {
        Ok(formatdoc!(
            r#"Name: {name} {default}
            Type: {vault_type}"#,
            name = self
                .name
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            default = if self.is_default { "(default)" } else { "" },
            vault_type = match self.config.is_aws() {
                true => "AWS KMS",
                false => "OCKAM",
            }
            .to_string()
            .color(OckamColor::PrimaryResource.color()),
        ))
    }
}
