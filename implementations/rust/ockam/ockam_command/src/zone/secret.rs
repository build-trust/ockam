use crate::cluster::common_args::HttpApiArgs;
use crate::cluster::utils::{get_api_client, get_cluster};
use crate::zone::common_args::{SecretsConfigArg, ZoneNameOrConfigArg};
use crate::{docs, node_command::InMemoryNodeCommand, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use base64ct::Encoding;
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use ockam_api::colors::color_primary;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::ai_platform::api::AiPlatformApi;
use ockam_api::{fmt_log, fmt_ok, fmt_warn};
use ockam_core::compat::collections::HashMap;
use ockam_node::Context;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const LONG_ABOUT: &str = include_str!("./static/secret/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/secret/after_long_help.txt");

/// Create a secret
#[derive(Clone, Debug, Args, Default)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct SecretCommand {
    #[command(flatten)]
    pub secrets_config: SecretsConfigArg,

    #[command(flatten)]
    pub zone: ZoneNameOrConfigArg,

    #[command(flatten)]
    pub http_api: HttpApiArgs,
}

#[derive(Clone)]
struct SecretNodeCommand {
    opts: CommandGlobalOpts,
    command: SecretCommand,
}

#[async_trait]
impl InMemoryNodeCommand for SecretNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        let use_http_api = self.command.http_api.use_http_api();
        let api_client = get_api_client(&node, use_http_api).await?;
        let cluster = get_cluster(ctx, &node).await?;
        let zone_name = self.command.zone.zone_name()?;

        // Push secrets
        self.command
            .push_secrets(ctx, &self.opts, &*api_client, &cluster, &zone_name)
            .await?;

        // List secrets
        let spinner = self.opts.terminal.spinner();
        if let Some(spinner) = &spinner {
            spinner.set_message("Listing secrets...");
        }
        let secrets = api_client
            .list_secrets(ctx, Some(&cluster), &zone_name)
            .await?;
        if let Some(spinner) = &spinner {
            spinner.finish_and_clear();
        }
        if secrets.is_empty() {
            self.opts.terminal.write_line(fmt_warn!(
                "No secrets defined for zone {} in cluster {}",
                color_primary(&zone_name),
                color_primary(&cluster)
            ))?;
        } else {
            self.opts.terminal.write_line(fmt_log!(
                "Secrets for zone {} in cluster {}: {}",
                color_primary(&zone_name),
                color_primary(&cluster),
                secrets
                    .iter()
                    .map(|s| format!("{}", color_primary(&s.name)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))?;
        }

        Ok(())
    }
}

#[async_trait]
impl Command for SecretCommand {
    const NAME: &'static str = "zone secret";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let command = SecretNodeCommand {
            opts: opts.clone(),
            command: self.clone(),
        };
        command.execute(ctx, opts.state.clone()).await?;
        Ok(())
    }
}

impl SecretCommand {
    pub async fn push_secrets(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
        api_client: &(dyn AiPlatformApi + Send + Sync + 'static),
        cluster: &str,
        zone_name: &str,
    ) -> Result<()> {
        let secrets = match self.parse_secrets_file()? {
            Some(secrets) => secrets,
            None => {
                return Ok(());
            }
        };

        // Delete existing secrets
        let spinner = opts.terminal.spinner();
        if let Some(spinner) = &spinner {
            spinner.set_message("Listing secrets...");
        }
        let secrets_to_remove = api_client
            .list_secrets(ctx, Some(cluster), zone_name)
            .await?;
        if !secrets_to_remove.is_empty() {
            if let Some(spinner) = &spinner {
                spinner.set_message("Deleting existing secrets...");
            }
        }
        for secret in &secrets_to_remove {
            api_client
                .delete_secret(ctx, Some(cluster), zone_name, &secret.name)
                .await?;
        }

        // Push secrets from file
        for secret in &secrets.0 {
            if let Some(spinner) = &spinner {
                spinner.set_message(format!(
                    "Creating secret {} to zone {} in cluster {}...",
                    color_primary(&secret.name),
                    color_primary(zone_name),
                    color_primary(cluster)
                ));
            }
            api_client
                .create_secret(ctx, Some(cluster), zone_name, &secret.name, &secret.fields)
                .await?;
        }

        if let Some(spinner) = &spinner {
            spinner.finish_and_clear();
        }
        opts.terminal.write_line(fmt_ok!(
            "Pushed secrets to zone {} in cluster {}",
            color_primary(zone_name),
            color_primary(cluster)
        ))?;

        Ok(())
    }

    fn parse_secrets_file(&self) -> Result<Option<Secrets>> {
        let path = match &self.secrets_config.secrets_config {
            Some(path) => path,
            None => {
                if std::path::Path::new("./secrets.yaml")
                    .try_exists()
                    .into_diagnostic()?
                {
                    "./secrets.yaml"
                } else if std::path::Path::new("./secrets.yml")
                    .try_exists()
                    .into_diagnostic()?
                {
                    "./secrets.yml"
                } else {
                    return Ok(None);
                }
            }
        };
        Ok(Some(Secrets::from_file(path)?))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Secrets(Vec<Secret>);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Secret {
    name: String,
    fields: HashMap<String, String>,
}

impl Secrets {
    fn from_contents(contents: &str) -> Result<Self> {
        let _self = if contents.starts_with("{") {
            serde_json::from_str::<Self>(contents)
                .map_err(|e| miette::miette!(format!("Failed to parse JSON secrets: {}", e)))?
        } else {
            serde_yaml::from_str::<Self>(contents)
                .map_err(|e| miette::miette!(format!("Failed to parse YAML secrets: {}", e)))?
        };
        _self.validate()?;
        Ok(_self)
    }

    fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .into_diagnostic()
            .wrap_err(format!(
                "Failed to read secrets file at {}",
                path.as_ref().display()
            ))?;
        Self::from_contents(&content)
    }

    fn validate(&self) -> Result<()> {
        // Key values must be base64 encoded
        for secret in &self.0 {
            for (key, value) in &secret.fields {
                if base64ct::Base64::decode_vec(value).is_err() {
                    return Err(miette!(format!(
                        "Invalid value for key '{}' in secret '{}'",
                        key, secret.name
                    ))
                    .wrap_err("Key's value must be base64 encoded"));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_file_with_content(content: &str) -> Result<NamedTempFile> {
        let mut file = NamedTempFile::new().into_diagnostic()?;
        file.write_all(content.as_bytes()).into_diagnostic()?;
        file.flush().into_diagnostic()?;
        Ok(file)
    }

    #[test]
    fn test_parse_yaml_direct_array() -> Result<()> {
        let yaml_content = r#"
- name: pg
  fields:
    username: dQ==
    password: cA==
"#;
        let file = create_temp_file_with_content(yaml_content)?;
        let secrets = Secrets::from_file(file.path())?;

        assert_eq!(secrets.0.len(), 1);
        assert_eq!(secrets.0[0].name, "pg");
        assert_eq!(
            secrets.0[0].fields.get("username"),
            Some(&"dQ==".to_string())
        );
        assert_eq!(
            secrets.0[0].fields.get("password"),
            Some(&"cA==".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_parse_yaml_multiple_secrets() -> Result<()> {
        let yaml_content = r#"
- name: pg
  fields:
    username: dQ==
    password: cA==
- name: redis
  fields:
    host: bG9jYWxob3N0
    port: NjM3OQ==
"#;
        let file = create_temp_file_with_content(yaml_content)?;
        let secrets = Secrets::from_file(file.path())?;

        assert_eq!(secrets.0.len(), 2);

        assert_eq!(secrets.0[0].name, "pg");
        assert_eq!(
            secrets.0[0].fields.get("username"),
            Some(&"dQ==".to_string())
        );

        assert_eq!(secrets.0[1].name, "redis");
        assert_eq!(
            secrets.0[1].fields.get("port"),
            Some(&"NjM3OQ==".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_parse_json_format() -> Result<()> {
        let json_content = r#"[
  {
    "name": "pg",
    "fields": {
      "username": "dQ==",
      "password": "cA=="
    }
  }
]"#;
        let file = create_temp_file_with_content(json_content)?;
        let secrets = Secrets::from_file(file.path())?;

        assert_eq!(secrets.0.len(), 1);
        assert_eq!(secrets.0[0].name, "pg");
        assert_eq!(
            secrets.0[0].fields.get("username"),
            Some(&"dQ==".to_string())
        );
        assert_eq!(
            secrets.0[0].fields.get("password"),
            Some(&"cA==".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_parse_invalid_yaml() -> Result<()> {
        let invalid_yaml = "this is not valid yaml";
        let file = create_temp_file_with_content(invalid_yaml)?;
        let result = Secrets::from_file(file.path());
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_secrets_validation_valid() -> Result<()> {
        // Use valid base64 encoded values
        let yaml_content = r#"
    - name: test-secret
      fields:
        key1: SGVsbG8gV29ybGQ=
        key2: QkFTRTY0
    "#;
        let file = create_temp_file_with_content(yaml_content)?;
        let secrets = Secrets::from_file(file.path())?;
        let validation_result = secrets.validate();
        assert!(validation_result.is_ok());
        Ok(())
    }

    #[test]
    fn test_secrets_validation_invalid() -> Result<()> {
        // Use one valid and one invalid base64 value
        let yaml_content = r#"
    - name: test-secret
      fields:
        kye1: SGVsbG8gV29ybGQ=
        key2: this-is-not-valid-base64!
    "#;
        let file = create_temp_file_with_content(yaml_content)?;
        let secrets = Secrets::from_file(file.path());
        assert!(secrets.is_err());
        Ok(())
    }

    #[test]
    fn test_secrets_validation_empty_value() -> Result<()> {
        // Empty string is valid base64
        let yaml_content = r#"
    - name: test-secret
      fields:
        empty: ""
    "#;
        let file = create_temp_file_with_content(yaml_content)?;
        let secrets = Secrets::from_file(file.path())?;
        let validation_result = secrets.validate();
        assert!(validation_result.is_ok());
        Ok(())
    }

    #[test]
    fn test_secrets_validation_multiple_secrets() -> Result<()> {
        // Multiple secrets with one invalid value in the second secret
        let yaml_content = r#"
    - name: first-secret
      fields:
        key1: SGVsbG8=
    - name: second-secret
      fields:
        key1: SGVsbG8=
        key2: not-base64!
    "#;
        let file = create_temp_file_with_content(yaml_content)?;
        let secrets = Secrets::from_file(file.path());
        assert!(secrets.is_err());
        Ok(())
    }
}
