use crate::{fmt_info, GlobalArgs, Terminal};
use clap::crate_version;
use colorful::Colorful;
use ockam_core::env::get_env_with_default;
use serde::Deserialize;
use std::env;

#[derive(Deserialize)]
pub struct UpgradeFile {
    #[serde(default = "default_upgrade_message")]
    pub upgrade_message: String,
    #[serde(default = "default_upgrade_message_macos")]
    pub upgrade_message_macos: String,
}

fn default_upgrade_message() -> String {
    "Check out the latest release at https://github.com/build-trust/ockam/releases".to_string()
}

fn default_upgrade_message_macos() -> String {
    "Run the following command to upgrade the Ockam Command: 'brew install build-trust/ockam/ockam'"
        .to_string()
}

pub fn check_if_an_upgrade_is_available(global_args: &GlobalArgs) {
    if upgrade_check_is_disabled() || global_args.test_argument_parser {
        return;
    }
    let url = format!(
        "https://github.com/build-trust/ockam/releases/download/ockam_v{}/upgrade.json",
        crate_version!()
    );
    if let Ok(r) = reqwest::blocking::get(url) {
        if let Ok(f) = r.json::<UpgradeFile>() {
            let terminal = Terminal::from(global_args);
            terminal
                .write_line(fmt_info!("{}", f.upgrade_message))
                .unwrap();
            if cfg!(target_os = "macos") {
                terminal
                    .write_line(fmt_info!("{}", f.upgrade_message_macos))
                    .unwrap();
            }
        }
    }
}

fn upgrade_check_is_disabled() -> bool {
    get_env_with_default("OCKAM_DISABLE_UPGRADE_CHECK", false).unwrap_or(false)
}
