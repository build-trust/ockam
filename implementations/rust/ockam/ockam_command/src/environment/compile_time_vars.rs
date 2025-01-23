use ockam_api::cli_state::OCKAM_HOME;
use ockam_api::orchestrator::{OCKAM_CONTROLLER_ADDRESS, OCKAM_CONTROLLER_IDENTIFIER};
use ockam_core::env::{get_env_with_default, FromString};

// Runtime environment variables names used in the brand.rs binary
pub const OCKAM_COMMAND_BIN_NAME: &str = "OCKAM_COMMAND_BIN_NAME";
pub const OCKAM_COMMAND_BRAND_NAME: &str = "OCKAM_COMMAND_BRAND_NAME";
pub const OCKAM_COMMAND_SUPPORT_EMAIL: &str = "OCKAM_COMMAND_SUPPORT_EMAIL";
pub const OCKAM_COMMANDS: &str = "OCKAM_COMMANDS";

// Compile time environment variables names used in the command binary
pub const OCKAM_DEVELOPER: &str = env!("OCKAM_DEVELOPER");
pub const BIN_NAME: &str = env!("OCKAM_COMMAND_BIN_NAME");
pub const BRAND_NAME: &str = env!("OCKAM_COMMAND_BRAND_NAME");
pub const SUPPORT_EMAIL: &str = env!("OCKAM_COMMAND_SUPPORT_EMAIL");
/// A comma separated list of commands that can be run
/// in the format `command1=customName,command2,command3`
pub const COMMANDS: &str = env!("OCKAM_COMMANDS");

pub fn load_compile_time_vars() {
    std::env::set_var(
        ockam_api::logs::env_variables::OCKAM_DEVELOPER,
        OCKAM_DEVELOPER,
    );
    if let Ok(home_dir) = get_env_with_default(OCKAM_HOME, env!("OCKAM_HOME").to_string()) {
        if !home_dir.is_empty() {
            std::env::set_var(OCKAM_HOME, home_dir);
        }
    }
    if let Ok(orchestrator_identifier) = get_env_with_default(
        OCKAM_CONTROLLER_IDENTIFIER,
        env!("OCKAM_CONTROLLER_IDENTITY_ID").to_string(),
    ) {
        if !orchestrator_identifier.is_empty() {
            std::env::set_var(OCKAM_CONTROLLER_IDENTIFIER, orchestrator_identifier);
        }
    }
    if let Ok(orchestrator_address) = get_env_with_default(
        OCKAM_CONTROLLER_ADDRESS,
        env!("OCKAM_CONTROLLER_ADDR").to_string(),
    ) {
        if !orchestrator_address.is_empty() {
            std::env::set_var(OCKAM_CONTROLLER_ADDRESS, orchestrator_address);
        }
    }
}

pub fn is_ockam_developer() -> bool {
    bool::from_string(OCKAM_DEVELOPER).unwrap_or(false)
}
