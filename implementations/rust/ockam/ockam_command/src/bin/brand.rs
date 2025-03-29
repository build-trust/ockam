use clap::{CommandFactory, Parser};
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, Result, WrapErr};
use ockam::identity::Identifier;
use ockam_api::colors::{color_primary, color_warn};
use ockam_api::terminal::INDENTATION;
use ockam_api::{fmt_log, fmt_warn};
use ockam_command::branding::compile_env_vars::*;
use ockam_command::entry_point::top_level_command_names;
use ockam_command::OckamCommand;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use std::path::{Path, PathBuf};

static CRATE_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("Couldn't get the value for the `CARGO_MANIFEST_DIR` env variable");
    Path::new(&crate_dir).to_path_buf()
});

static BIN_DIR: Lazy<PathBuf> = Lazy::new(|| CRATE_DIR.join("src/bin"));

#[derive(Debug, Parser, Clone)]
#[command(arg_required_else_help = true)]
struct Args {
    /// Path to the configuration file or inline contents
    #[clap(long)]
    configuration: String,

    /// Prints the processed configuration without building the binaries
    #[clap(long)]
    dry_run: bool,
}

/// Builds the binaries with the passed configuration:
/// `cargo run --bin brand -- --configuration ./path/to/config.yaml`
/// `cargo run --bin brand -- --configuration "{bin1: {brand_name: "Name"}}"`
///
/// or with cross:
/// `cross run --target aarch64-unknown-linux-gnu --bin brand -- brand --configuration ./path/to/config.yaml`
fn main() -> Result<()> {
    let args = Args::parse();

    // Load the configuration
    let config: Config = match serde_yaml::from_str(&args.configuration) {
        Ok(config) => config,
        Err(_) => {
            let config = std::fs::read_to_string(&args.configuration).into_diagnostic()?;
            serde_yaml::from_str(&config).into_diagnostic()?
        }
    };

    // Build the binaries
    let cmd = OckamCommand::command();
    let top_level_commands = top_level_command_names(&cmd);
    for (bin_name, brand_config) in config.items {
        build_binary(bin_name, brand_config, &top_level_commands, args.dry_run)?;
    }
    Ok(())
}

/// Builds the binary with the passed settings
fn build_binary(
    bin_name: String,
    brand_settings: Brand,
    top_level_commands: &[String],
    dry_run: bool,
) -> Result<()> {
    brand_settings.validate()?;

    eprintln!(
        "{}\n{brand_settings}",
        fmt_log!("Building binary {} with", color_primary(&bin_name))
    );

    let commands = brand_settings.commands(top_level_commands);
    let brand_name = brand_settings.brand_name(&bin_name);
    let home_dir = brand_settings.home_dir(&bin_name);

    if dry_run {
        return Ok(());
    }

    let bin_path = create_temporary_binary_file(&bin_name)?;
    let mut cmd = std::process::Command::new("cargo");
    cmd.args(["build", "--bin", &bin_name]);
    if let Some(build_args) = &brand_settings.build_args {
        cmd.args(build_args);
    }

    cmd.envs([
        (COMPILE_OCKAM_DEVELOPER, "false".to_string()),
        (
            COMPILE_OCKAM_COMMAND_SUPPORT_EMAIL,
            brand_settings.support_email,
        ),
        (COMPILE_OCKAM_COMMAND_BIN_NAME, bin_name.clone()),
        (COMPILE_OCKAM_COMMAND_BRAND_NAME, brand_name),
        (COMPILE_OCKAM_HOME, home_dir),
        (
            COMPILE_OCKAM_CONTROLLER_IDENTIFIER,
            brand_settings
                .orchestrator_identifier
                .map(|i| i.to_string())
                .unwrap_or_default(),
        ),
        (
            COMPILE_OCKAM_CONTROLLER_ADDRESS,
            brand_settings.orchestrator_address.unwrap_or_default(),
        ),
        (COMPILE_OCKAM_COMMANDS, commands),
    ]);

    let res = cmd
        .status()
        .into_diagnostic()
        .wrap_err(format!("failed to build {bin_name} binary"));
    let _ = std::fs::remove_file(bin_path);
    res?;

    Ok(())
}

/// Copies the `./src/bin/ockam.rs` file into a new file using the passed name
/// This file will be used as the entry point for the binary
fn create_temporary_binary_file(bin_name: &str) -> Result<PathBuf> {
    let src = BIN_DIR.join("ockam.rs");
    let dst = BIN_DIR.join(format!("{bin_name}.rs"));
    std::fs::copy(src, &dst).into_diagnostic()?;
    Ok(dst)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Config {
    #[serde(flatten)]
    items: BTreeMap<String, Brand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct Brand {
    support_email: String,
    brand_name: Option<String>,
    home_dir: Option<String>,
    orchestrator_identifier: Option<Identifier>,
    orchestrator_address: Option<String>,
    project_address_template: Option<String>,
    commands: Option<Vec<Command>>,
    build_args: Option<Vec<String>>,
}

impl Brand {
    fn validate(&self) -> Result<()> {
        if let Some(template) = &self.project_address_template {
            if !template.contains("<id>") {
                return Err(miette!("project_address_template must contain the '<id>' placeholder, e.g '<id>.hostname.io'"));
            }
        }
        Ok(())
    }

    fn brand_name(&self, bin_name: &str) -> String {
        match &self.brand_name {
            Some(b) => b.clone(),
            None => {
                let mut brand_name = bin_name.to_string();
                brand_name[..1].make_ascii_uppercase();
                brand_name
            }
        }
    }

    fn home_dir(&self, bin_name: &str) -> String {
        match &self.home_dir {
            Some(home_dir) => home_dir.clone(),
            None => Path::new("$HOME")
                .join(bin_name)
                .to_string_lossy()
                .to_string(),
        }
    }

    fn commands(&self, top_level_commands: &[String]) -> String {
        match &self.commands {
            None => String::new(),
            Some(commands) => {
                let process_command_name = |name: &str, custom_name: Option<&str>| {
                    if !top_level_commands.iter().any(|t| t == name) {
                        eprintln!(
                            "{}",
                            fmt_warn!(
                                "Command {} is not a top level command, it can't be renamed or hidden. Skipping...",
                                color_primary(name)
                            )
                        );
                        return None;
                    }

                    // replace _ and - with space to support writing
                    // commands as "node create", "node-create" or "node_create
                    let name = match custom_name {
                        Some(custom_name) => format!("{}={}", name, custom_name),
                        None => name.to_string(),
                    };
                    Some(name.replace("_", " ").replace("-", " "))
                };

                // A comma separated list of commands in the format `command1=customName,command2,command3`
                commands
                    .iter()
                    .filter_map(|c| match c {
                        Command::Simple(c) => process_command_name(c, None),
                        Command::Mapped(map) => map
                            .iter()
                            .map(|(k, v)| process_command_name(k, Some(v)))
                            .collect::<Option<Vec<String>>>()
                            .map(|v| v.join(",")),
                    })
                    .collect::<Vec<String>>()
                    .join(",")
            }
        }
    }
}

impl Display for Brand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{}",
            fmt_log!(
                "{INDENTATION}support email: {}",
                color_primary(&self.support_email)
            )
        )?;
        if let Some(brand_name) = &self.brand_name {
            writeln!(
                f,
                "{}",
                fmt_log!("{INDENTATION}brand name: {}", color_primary(brand_name))
            )?;
        }
        if let Some(home_dir) = &self.home_dir {
            writeln!(
                f,
                "{}",
                fmt_log!("{INDENTATION}home dir: {}", color_primary(home_dir))
            )?;
        }
        if let Some(orchestrator_identifier) = &self.orchestrator_identifier {
            writeln!(
                f,
                "{}",
                fmt_log!(
                    "{INDENTATION}orchestrator identifier: {}",
                    color_primary(orchestrator_identifier)
                )
            )?;
        }
        if let Some(orchestrator_address) = &self.orchestrator_address {
            writeln!(
                f,
                "{}",
                fmt_log!(
                    "{INDENTATION}orchestrator address: {}",
                    color_primary(orchestrator_address)
                )
            )?;
        }
        if let Some(project_address_template) = &self.project_address_template {
            writeln!(
                f,
                "{}",
                fmt_log!(
                    "{INDENTATION}project address template: {}",
                    color_primary(project_address_template)
                )
            )?;
        }
        if let Some(commands) = &self.commands {
            writeln!(f, "{}", fmt_log!("{INDENTATION}commands:"))?;
            for command in commands {
                match command {
                    Command::Simple(c) => {
                        writeln!(
                            f,
                            "{}",
                            fmt_log!("{INDENTATION}{INDENTATION}{}", color_primary(c))
                        )?;
                    }
                    Command::Mapped(map) => {
                        for (k, v) in map {
                            writeln!(
                                f,
                                "{}",
                                fmt_log!(
                                    "{INDENTATION}{INDENTATION}{}={}",
                                    color_primary(k),
                                    color_warn(v)
                                )
                            )?;
                        }
                    }
                }
            }
        }
        if let Some(build_args) = &self.build_args {
            writeln!(
                f,
                "{}",
                fmt_log!(
                    "{INDENTATION}build args: {}",
                    color_primary(build_args.join(" "))
                )
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum Command {
    Simple(String),
    Mapped(HashMap<String, String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_file() {
        let config = r#"
          bin1:
            support_email: bin@support.io
            brand_name: Brand1
            home_dir: /home/brand1
            orchestrator_identifier: I25242aa3d4a7b5aa986fb2bec15b3780aad0530660e5e5a46c7f9ce429e9ec99
            orchestrator_address: brand1.network
            project_address_template: <id>.brand1.network
            commands:
              - node_list
              - "node create": "init"
          bin2:
            support_email: bin2@support.io
            build_args:
              - --release
              - --target
              - armv7-unknown-linux-gnueabihf
        "#;
        let parsed: Config = serde_yaml::from_str(config).unwrap();
        assert_eq!(parsed.items.len(), 2);
        assert_eq!(parsed.items["bin1"].brand_name.as_deref(), Some("Brand1"));
        assert_eq!(parsed.items["bin2"].brand_name, None);

        // No defaults used, should be the same as parsed
        let bin1 = &parsed.items["bin1"];
        assert_eq!(&parsed.items["bin1"], bin1);

        // Check bin2 defaults
        let bin2 = &parsed.items["bin2"];
        assert_eq!(bin2.support_email, "bin2@support.io");
        assert_eq!(bin2.brand_name, None);
        assert_eq!(bin2.brand_name("bin2"), "Bin2");
        assert_eq!(bin2.home_dir.as_ref(), None);
        assert_eq!(bin2.home_dir("bin2"), "$HOME/bin2");
        assert_eq!(bin2.orchestrator_identifier, None);
        assert_eq!(bin2.orchestrator_address.as_deref(), None);
        assert_eq!(bin2.commands.as_deref(), None);
        assert_eq!(
            bin2.build_args.clone().unwrap(),
            vec!["--release", "--target", "armv7-unknown-linux-gnueabihf",]
        );
    }

    #[test]
    fn test_validation_errors() {
        let no_support_email = r#"
          bin:
            brand_name: Brand
        "#;
        serde_yaml::from_str::<Config>(no_support_email)
            .expect_err("The support_email field is required");

        let invalid_identifier = r#"
          bin:
            support_email: bin@support.io
            orchestrator_identifier: identifier
        "#;
        serde_yaml::from_str::<Config>(invalid_identifier)
            .expect_err("Should fail to parse the orchestrator_identifier field");

        let invalid_template = r#"
            bin:
                support_email: bin@support.io
                project_address_template: hostname.io"
        "#;
        let config = serde_yaml::from_str::<Config>(invalid_template).unwrap();
        config.items["bin"]
            .validate()
            .expect_err("Should fail to validate the project_address_template field");
    }
}
