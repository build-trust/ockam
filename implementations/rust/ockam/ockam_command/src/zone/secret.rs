use crate::cluster::common_args::HttpApiArgs;
use crate::cluster::utils::{get_api_client, get_cluster};
use crate::zone::common_args::{SecretsConfigArg, ZoneNameOrConfigArg};
use crate::{docs, node_command::InMemoryNodeCommand, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use base64ct::Encoding;
use clap::Args;
use colorful::Colorful;
use miette::{IntoDiagnostic, WrapErr};
use ockam_api::colors::color_primary;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::ai_platform::api::AiPlatformApi;
use ockam_api::{fmt_log, fmt_ok, fmt_warn};
use ockam_core::compat::collections::HashMap;
use ockam_node::Context;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    pub zone: ZoneNameOrConfigArg,

    #[command(flatten)]
    pub secrets_config: SecretsConfigArg,

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
        let secrets = match self.parse_secrets_config()? {
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

    fn parse_secrets_config(&self) -> Result<Option<Secrets>> {
        match self.secrets_config.contents()? {
            Some(contents) => Ok(Some(Secrets::from_contents(&contents)?)),
            None => Ok(None),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecretsYaml(BTreeMap<String, String>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Secrets(Vec<Secret>);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Secret {
    name: String,
    fields: HashMap<String, String>,
}

impl Secrets {
    pub(crate) const SECRET_NAME: &'static str = "secret";

    fn from_contents(contents: &str) -> Result<Self> {
        let mut _self = if let Ok(parsed_yaml) = Self::parse_contents::<SecretsYaml>(contents) {
            Self::from_parsed_yaml(parsed_yaml)?
        } else {
            Self::parse_contents::<Self>(contents)?
        };
        _self.encode_secret_values()?;
        Ok(_self)
    }

    fn parse_contents<T>(contents: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        if contents.starts_with("{") {
            serde_json::from_str(contents)
                .map_err(|e| miette::miette!(format!("Failed to parse JSON: {}", e)))
        } else {
            serde_yaml::from_str(contents)
                .map_err(|e| miette::miette!(format!("Failed to parse YAML: {}", e)))
        }
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

    fn from_parsed_yaml(parsed_yaml: SecretsYaml) -> Result<Self> {
        let mut secrets = Vec::new();
        let mut secret = Secret {
            name: Self::SECRET_NAME.to_string(),
            fields: HashMap::from([]),
        };
        for (name, value) in parsed_yaml.0 {
            secret.fields.insert(name, value);
        }
        secrets.push(secret);
        Ok(Self(secrets))
    }

    fn encode_secret_values(&mut self) -> Result<()> {
        for secret in &mut self.0 {
            for value in secret.fields.values_mut() {
                *value = base64ct::Base64::encode_string(value.as_bytes());
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

    mod simplified_format {
        use super::*;

        #[test]
        fn test_parse_yaml() -> Result<()> {
            let yaml_content = r#"
            pg_username: u
            pg_password: p
            "#;
            let file = create_temp_file_with_content(yaml_content)?;
            let secrets = Secrets::from_file(file.path())?;
            let _secrets_as_str = serde_yaml::to_string(&secrets)
                .into_diagnostic()
                .wrap_err("Failed to serialize secrets to YAML")?;

            assert_eq!(secrets.0.len(), 1);
            let secret = secrets.0.first().unwrap();
            assert_eq!(secret.name, Secrets::SECRET_NAME);
            assert_eq!(secret.fields.get("pg_username"), Some(&"dQ==".to_string()));
            assert_eq!(secret.fields.get("pg_password"), Some(&"cA==".to_string()));
            Ok(())
        }
    }

    mod raw_format {
        use super::*;

        #[test]
        fn test_parse_yaml_direct_array() -> Result<()> {
            let yaml_content = r#"
            - name: pg
              fields:
                username: u
                password: p
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
                username: u
                password: p
            - name: redis
              fields:
                host: localhost
                port: 6379
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
                  "username": "u",
                  "password": "p"
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
    }

    #[test]
    fn test_parse_invalid_yaml() -> Result<()> {
        let invalid_yaml = "this is not valid yaml";
        let file = create_temp_file_with_content(invalid_yaml)?;
        let result = Secrets::from_file(file.path());
        assert!(result.is_err());
        Ok(())
    }

    mod parsing_from_arg {
        use super::*;
        use crate::zone::common_args::SecretsConfigArg;

        #[test]
        fn test_parse_secrets_from_inline_yaml() -> Result<()> {
            let yaml_content = r#"
            pg_username: testuser
            pg_password: testpass
            "#;
            let secrets_config = SecretsConfigArg {
                secrets_config: Some(yaml_content.to_string()),
            };
            let command = SecretCommand {
                secrets_config,
                ..Default::default()
            };

            let result = command.parse_secrets_config()?;
            assert!(result.is_some());

            let secrets = result.unwrap();
            assert_eq!(secrets.0.len(), 1);
            let secret = &secrets.0[0];
            assert_eq!(secret.name, Secrets::SECRET_NAME);
            assert!(secret.fields.contains_key("pg_username"));
            assert!(secret.fields.contains_key("pg_password"));

            Ok(())
        }

        #[test]
        fn test_parse_secrets_from_inline_json() -> Result<()> {
            let json_content = r#"[
              {
                "name": "test-secret",
                "fields": {
                  "username": "testuser",
                  "password": "testpass"
                }
              }
            ]"#;
            let secrets_config = SecretsConfigArg {
                secrets_config: Some(json_content.to_string()),
            };
            let command = SecretCommand {
                secrets_config,
                ..Default::default()
            };

            let result = command.parse_secrets_config()?;
            assert!(result.is_some());

            let secrets = result.unwrap();
            assert_eq!(secrets.0.len(), 1);
            assert_eq!(secrets.0[0].name, "test-secret");

            Ok(())
        }

        #[test]
        fn test_parse_secrets_file_path_takes_precedence() -> Result<()> {
            let yaml_content = r#"
            file_username: fileuser
            file_password: filepass
            "#;
            let file = create_temp_file_with_content(yaml_content)?;
            let secrets_config = SecretsConfigArg {
                secrets_config: Some(file.path().to_string_lossy().to_string()),
            };
            let command = SecretCommand {
                secrets_config,
                ..Default::default()
            };

            let result = command.parse_secrets_config()?;
            assert!(result.is_some());

            let secrets = result.unwrap();
            let secret = &secrets.0[0];
            assert!(secret.fields.contains_key("file_username"));

            Ok(())
        }

        #[test]
        fn test_parse_secrets_invalid_inline_content() {
            let invalid_content = "invalid: yaml: content: [";
            let secrets_config = SecretsConfigArg {
                secrets_config: Some(invalid_content.to_string()),
            };
            let command = SecretCommand {
                secrets_config,
                ..Default::default()
            };

            let result = command.parse_secrets_config();
            assert!(result.is_err());
        }
    }
}
