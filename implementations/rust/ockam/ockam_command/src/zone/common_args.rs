use crate::docs;
use crate::util::parsers::alphanumeric_parser;
use crate::zone::zone_config::ZoneConfig;
use clap::Args;
use miette::{miette, Context as _, IntoDiagnostic};
use ockam_api::orchestrator::ai_platform::api::AiPlatformApi;
use ockam_node::Context;
use std::collections::BTreeMap;
use std::path::PathBuf;

const ZONE_CONFIG_HELP: &str = include_str!("./static/common_args/zone_config.txt");
const ZONE_NAME_HELP: &str = "The name of the Zone";

#[derive(Clone, Debug, Args, Default)]
pub struct ZoneConfigArg {
    #[arg(long, visible_alias = "config", help = ZONE_CONFIG_HELP)]
    pub zone_config: Option<String>,
}

impl ZoneConfigArg {
    pub fn new(zone_config: Option<String>) -> Self {
        Self { zone_config }
    }

    pub fn zone_config_path(&self) -> crate::Result<PathBuf> {
        match &self.zone_config {
            Some(path) => {
                let path = PathBuf::from(path);
                if path.exists() {
                    Ok(path)
                } else {
                    Err(miette!(
                        "Zone config file not found at the specified path: {}",
                        path.display()
                    ))
                }
            }
            None => {
                let paths = vec![PathBuf::from("./ockam.yaml"), PathBuf::from("./ockam.yml")];
                for path in paths {
                    if path.try_exists().into_diagnostic()? {
                        return Ok(path);
                    }
                }
                Err(miette!(
                    "Zone config file not found. Please provide a zone name or a zone config file path."
                ))
            }
        }
    }

    /// Parse the zone config contents as an inline string or from a file.
    pub fn zone_config(&self) -> crate::Result<ZoneConfig> {
        let contents = match self.zone_config_path() {
            Ok(path) => {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    contents
                } else {
                    return Err(miette!("Zone config file not found or is not readable"));
                }
            }
            Err(err) => {
                if let Some(contents) = &self.zone_config {
                    contents.clone()
                } else {
                    return Err(err);
                }
            }
        };
        ZoneConfig::from_contents(&contents)
            .map_err(|e| miette!("Failed to parse zone config: {}", e))
    }

    pub fn zone_name(&self) -> crate::Result<String> {
        Ok(self.zone_config()?.name)
    }
}

#[derive(Clone, Debug, Args, Default)]
#[group(multiple = false)]
pub struct ZoneNameOrConfigArg {
    #[arg(value_parser = alphanumeric_parser, help = ZONE_NAME_HELP)]
    pub zone_name: Option<String>,

    #[arg(long, visible_alias = "config", help = ZONE_CONFIG_HELP)]
    pub zone_config: Option<String>,
}

impl From<ZoneConfigArg> for ZoneNameOrConfigArg {
    fn from(zone_config: ZoneConfigArg) -> Self {
        Self {
            zone_name: zone_config.zone_name().ok(),
            zone_config: zone_config.zone_config,
        }
    }
}

impl From<ZoneNameLongOrConfigArg> for ZoneNameOrConfigArg {
    fn from(zone_name_or_config: ZoneNameLongOrConfigArg) -> Self {
        Self {
            zone_name: zone_name_or_config.zone_name,
            zone_config: zone_name_or_config.zone_config,
        }
    }
}

impl ZoneNameOrConfigArg {
    pub fn from_zone_name(zone_name: String) -> Self {
        Self {
            zone_name: Some(zone_name),
            zone_config: None,
        }
    }

    pub fn zone_name(&self) -> crate::Result<String> {
        if let Some(zone_name) = &self.zone_name {
            return Ok(zone_name.clone());
        }
        let zone_config = ZoneConfigArg::new(self.zone_config.clone());
        zone_config.zone_name()
    }

    pub fn zone_config(&self) -> crate::Result<ZoneConfig> {
        let zone_config = ZoneConfigArg::new(self.zone_config.clone());
        zone_config.zone_config()
    }
}

#[derive(Clone, Debug, Args, Default)]
#[group(multiple = false)]
pub struct ZoneNameLongOrConfigArg {
    #[arg(long = "zone", value_parser = alphanumeric_parser, help = ZONE_NAME_HELP)]
    pub zone_name: Option<String>,

    #[arg(long, visible_alias = "config", help = ZONE_CONFIG_HELP)]
    pub zone_config: Option<String>,
}

impl ZoneNameLongOrConfigArg {
    pub fn zone_name(&self) -> crate::Result<String> {
        ZoneNameOrConfigArg::from(self.clone()).zone_name()
    }
}

#[derive(Clone, Debug, Args, Default)]
pub struct SecretsConfigArg {
    /// The path to the secrets file, in yaml or json format.
    /// If not set, the `./secrets.yaml` file from the current directory will be used.
    /// If no file is found, the command will just list the existing secrets.
    #[arg(long, visible_alias = "secrets")]
    pub secrets_config: Option<String>,
}

#[derive(Clone, Debug, Args, Default)]
pub struct EnrollmentTicketConfigArg {
    #[arg(long, env = "ENROLLMENT_TICKET", value_name = "ENROLLMENT TICKET")]
    #[arg(help = docs::about("\
    A path, URL or inlined hex-encoded enrollment ticket to use for the Ockam Identity associated to this node. \
    If omitted, one will be created automatically with default attributes
    "))]
    pub enrollment_ticket: Option<String>,
}

impl EnrollmentTicketConfigArg {
    pub async fn get(
        &self,
        ctx: &Context,
        api_client: &(dyn AiPlatformApi + Send + Sync + 'static),
        cluster: &str,
        zone_name: &str,
        relay: Option<String>,
    ) -> crate::Result<String> {
        if let Some(t) = &self.enrollment_ticket {
            return Ok(t.clone());
        }
        api_client
            .create_enrollment_ticket(ctx, Some(cluster), zone_name, BTreeMap::default(), relay)
            .await.wrap_err("Failed to generate an enrollment ticket for the inlet. Please provide one with the --enrollment-ticket argument")
    }
}

#[derive(Clone, Debug, Args, Default)]
pub struct DockerBuildArgs {
    /// Whether to use the Docker cache when building the image.
    /// It can be set using the `OCKAM_DOCKER_NO_CACHE` environment variable.
    #[arg(long, env = "OCKAM_DOCKER_NO_CACHE")]
    pub no_cache: bool,

    /// Whether to build the image with the `--pull` option.
    /// It can be set using the `OCKAM_DOCKER_NO_PULL` environment variable.
    #[arg(long, hide = true, env = "OCKAM_DOCKER_NO_PULL")]
    pub no_pull: bool,
}

#[derive(Clone, Debug, Args, Default)]
pub struct ZoneInletsArgs {
    /// Skip the creation of the inlet to the http outlet.
    #[arg(long)]
    pub no_http: bool,

    /// Skip the creation of the inlet to the logs outlet.
    #[arg(long)]
    pub no_logs: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    mod zone_config_arg {
        use super::*;

        #[test]
        fn test_zone_config_from_inline_string() {
            let yaml_content = serde_yaml::to_string(&ZoneConfig::default()).unwrap();
            let zone_config_arg = ZoneConfigArg::new(Some(yaml_content.to_string()));
            let result = zone_config_arg.zone_config();

            assert!(result.is_ok());
            let config = result.unwrap();
            assert_eq!(config.name, "zone");
        }

        #[test]
        fn test_zone_config_from_file_path() {
            let temp_dir = TempDir::new().unwrap();
            let config_file_path = temp_dir.path().join("zone_config.yaml");

            let yaml_content = serde_yaml::to_string(&ZoneConfig::default()).unwrap();
            fs::write(&config_file_path, yaml_content).unwrap();

            let zone_config_arg =
                ZoneConfigArg::new(Some(config_file_path.to_string_lossy().to_string()));
            let result = zone_config_arg.zone_config();

            assert!(result.is_ok());
            let config = result.unwrap();
            assert_eq!(config.name, "zone");
        }

        #[test]
        fn test_zone_config_invalid_yaml() {
            let invalid_yaml = "invalid: yaml: content: [";
            let zone_config_arg = ZoneConfigArg::new(Some(invalid_yaml.to_string()));
            let result = zone_config_arg.zone_config();

            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Failed to parse zone config"));
        }

        #[test]
        fn test_zone_config_no_config_provided() {
            let zone_config_arg = ZoneConfigArg::new(None);
            let result = zone_config_arg.zone_config();

            // Should fail when no config is provided and no default files exist
            assert!(result.is_err());
        }
    }
}
