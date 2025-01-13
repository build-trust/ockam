use ockam_api::cli_state::OCKAM_HOME;
use ockam_api::cloud::{OCKAM_CONTROLLER_ADDR, OCKAM_CONTROLLER_IDENTITY_ID};
use ockam_core::env::get_env_with_default;

pub const OCKAM_COMMAND_BIN_NAME: &str = "OCKAM_COMMAND_BIN_NAME";
pub const OCKAM_COMMAND_BRAND_NAME: &str = "OCKAM_COMMAND_BRAND_NAME";
pub const OCKAM_COMMAND_SUPPORT_EMAIL: &str = "OCKAM_COMMAND_SUPPORT_EMAIL";

pub const BIN_NAME: &str = env!("OCKAM_COMMAND_BIN_NAME");
pub const BRAND_NAME: &str = env!("OCKAM_COMMAND_BRAND_NAME");
pub const SUPPORT_EMAIL: &str = env!("OCKAM_COMMAND_SUPPORT_EMAIL");

pub fn load_compile_time_vars() {
    std::env::set_var(OCKAM_COMMAND_BIN_NAME, BIN_NAME);
    std::env::set_var(OCKAM_COMMAND_BRAND_NAME, BRAND_NAME);
    std::env::set_var(OCKAM_COMMAND_SUPPORT_EMAIL, SUPPORT_EMAIL);
    if let Ok(home_dir) = get_env_with_default(OCKAM_HOME, env!("OCKAM_HOME").to_string()) {
        if !home_dir.is_empty() {
            std::env::set_var(OCKAM_HOME, home_dir);
        }
    }
    if let Ok(orchestrator_identifier) = get_env_with_default(
        OCKAM_CONTROLLER_IDENTITY_ID,
        env!("OCKAM_CONTROLLER_IDENTITY_ID").to_string(),
    ) {
        if !orchestrator_identifier.is_empty() {
            std::env::set_var(OCKAM_CONTROLLER_IDENTITY_ID, orchestrator_identifier);
        }
    }
    if let Ok(orchestrator_address) = get_env_with_default(
        OCKAM_CONTROLLER_ADDR,
        env!("OCKAM_CONTROLLER_ADDR").to_string(),
    ) {
        if !orchestrator_address.is_empty() {
            std::env::set_var(OCKAM_CONTROLLER_ADDR, orchestrator_address);
        }
    }
}
