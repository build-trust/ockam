use miette::{miette, IntoDiagnostic, Result, WrapErr};
use ockam_api::cli_state::OCKAM_HOME;
use ockam_api::cloud::{OCKAM_CONTROLLER_ADDR, OCKAM_CONTROLLER_IDENTITY_ID};
use ockam_api::colors::color_primary;
use ockam_api::fmt_log;
use ockam_api::terminal::PADDING;
use ockam_command::{
    OCKAM_COMMAND_BIN_NAME, OCKAM_COMMAND_BRAND_NAME, OCKAM_COMMAND_SUPPORT_EMAIL,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::{Path, PathBuf};

static CRATE_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("Couldn't get the value for the `CARGO_MANIFEST_DIR` env variable");
    Path::new(&crate_dir).to_path_buf()
});

static BIN_DIR: Lazy<PathBuf> = Lazy::new(|| CRATE_DIR.join("src/bin"));

/// Builds the binaries with the passed configuration
/// How to run:
/// `cargo run --bin brand ./path/to/config.yaml --release`
/// `cargo run --bin brand "{bin1: {brand_name: "Name"}}"`
fn main() -> Result<()> {
    // first argument: inline config or path to config file
    let config = std::env::args().nth(1).ok_or(miette!(
        "Provide at least one argument with binaries configuration"
    ))?;
    let mut config: Config = match serde_yaml::from_str(&config) {
        Ok(config) => config,
        Err(_) => {
            let config = std::fs::read_to_string(&config).into_diagnostic()?;
            serde_yaml::from_str(&config).into_diagnostic()?
        }
    };
    config.process_defaults()?;

    // build the binaries
    for (bin_name, brand_config) in config.items {
        build_binary(&bin_name, brand_config)?;
    }
    Ok(())
}

/// Builds the binary with the passed settings
fn build_binary(bin_name: &str, brand_settings: Brand) -> Result<()> {
    eprintln!(
        "{}\n{brand_settings}",
        fmt_log!("Building binary {} with", color_primary(bin_name))
    );

    let bin_path = create_temporary_binary_file(bin_name)?;
    let mut cmd = std::process::Command::new("cargo");
    cmd.args(["build", "--bin", bin_name]);
    cmd.env(OCKAM_COMMAND_BIN_NAME, bin_name);
    cmd.env(OCKAM_COMMAND_SUPPORT_EMAIL, &brand_settings.support_email);
    if let Some(brand_name) = brand_settings.brand_name {
        cmd.env(OCKAM_COMMAND_BRAND_NAME, brand_name);
    }
    if let Some(home_dir) = brand_settings.home_dir {
        cmd.env(OCKAM_HOME, home_dir);
    }
    if let Some(orchestrator_identifier) = brand_settings.orchestrator_identifier {
        cmd.env(OCKAM_CONTROLLER_IDENTITY_ID, orchestrator_identifier);
    }
    if let Some(orchestrator_address) = brand_settings.orchestrator_address {
        cmd.env(OCKAM_CONTROLLER_ADDR, orchestrator_address);
    }
    if let Some(build_args) = brand_settings.build_args {
        cmd.args(build_args.split_whitespace());
    }

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

impl Config {
    fn process_defaults(&mut self) -> Result<()> {
        for (bin_name, brand) in self.items.iter_mut() {
            // Default brand_name to capitalized bin_name
            if brand.brand_name.is_none() {
                let mut brand_name = bin_name.to_string();
                brand_name[..1].make_ascii_uppercase();
                brand.brand_name = Some(brand_name);
            }

            // Default home_dir to $HOME/.{bin_name}
            if brand.home_dir.is_none() {
                brand.home_dir = Some(
                    Path::new("$HOME")
                        .join(format!(".{bin_name}"))
                        .to_str()
                        .ok_or(miette!("Failed to convert path to string"))?
                        .to_string(),
                )
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct Brand {
    support_email: String,
    brand_name: Option<String>,
    home_dir: Option<String>,
    orchestrator_identifier: Option<String>,
    orchestrator_address: Option<String>,
    build_args: Option<String>,
}

impl Display for Brand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{}",
            fmt_log!(
                "{PADDING}support email {}",
                color_primary(&self.support_email)
            )
        )?;
        if let Some(brand_name) = &self.brand_name {
            writeln!(
                f,
                "{}",
                fmt_log!("{PADDING}brand name {}", color_primary(brand_name))
            )?;
        }
        if let Some(home_dir) = &self.home_dir {
            writeln!(
                f,
                "{}",
                fmt_log!("{PADDING}home dir {}", color_primary(home_dir))
            )?;
        }
        if let Some(orchestrator_identifier) = &self.orchestrator_identifier {
            writeln!(
                f,
                "{}",
                fmt_log!(
                    "{PADDING}orchestrator identifier {}",
                    color_primary(orchestrator_identifier)
                )
            )?;
        }
        if let Some(orchestrator_address) = &self.orchestrator_address {
            writeln!(
                f,
                "{}",
                fmt_log!(
                    "{PADDING}orchestrator address {}",
                    color_primary(orchestrator_address)
                )
            )?;
        }
        if let Some(build_args) = &self.build_args {
            writeln!(
                f,
                "{}",
                fmt_log!("{PADDING}build args {}", color_primary(build_args))
            )?;
        }
        Ok(())
    }
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
            orchestrator_identifier: brand1
            orchestrator_address: brand1.network
          bin2:
            support_email: bin2@support.io
            brand_name: Brand2
        "#;
        let parsed: Config = serde_yaml::from_str(config).unwrap();
        assert_eq!(parsed.items.len(), 2);
        assert_eq!(parsed.items["bin1"].brand_name.as_deref(), Some("Brand1"));
        assert_eq!(parsed.items["bin2"].support_email, "bin2@support.io");
        assert_eq!(parsed.items["bin2"].brand_name.as_deref(), Some("Brand2"));

        let mut processed = parsed.clone();
        processed.process_defaults().unwrap();
        assert_eq!(parsed.items["bin1"], processed.items["bin1"]);

        let bin2 = &processed.items["bin2"];
        assert_eq!(bin2.support_email, "bin2@support.io");
        assert_eq!(bin2.brand_name.as_deref(), Some("Brand2"));
        assert_eq!(bin2.home_dir.as_ref().unwrap(), "$HOME/.bin2");
        assert_eq!(bin2.orchestrator_identifier.as_deref(), None);
        assert_eq!(bin2.orchestrator_address.as_deref(), None);
    }
}
