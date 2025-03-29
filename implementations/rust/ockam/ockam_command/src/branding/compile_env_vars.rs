use ockam_api::cli_state::OCKAM_HOME;
use ockam_api::orchestrator::{OCKAM_CONTROLLER_ADDRESS, OCKAM_CONTROLLER_IDENTIFIER};
use ockam_core::env::{get_env_ignore_error, get_env_with_default, FromString};
use once_cell::sync::Lazy;

// Runtime environment variables names used in the brand.rs binary
pub const COMPILE_OCKAM_DEVELOPER: &str = "COMPILE_OCKAM_DEVELOPER";
pub const COMPILE_OCKAM_HOME: &str = "COMPILE_OCKAM_HOME";
pub const COMPILE_OCKAM_COMMAND_BIN_NAME: &str = "COMPILE_OCKAM_COMMAND_BIN_NAME";
pub const COMPILE_OCKAM_COMMAND_BRAND_NAME: &str = "COMPILE_OCKAM_COMMAND_BRAND_NAME";
pub const COMPILE_OCKAM_COMMAND_SUPPORT_EMAIL: &str = "COMPILE_OCKAM_COMMAND_SUPPORT_EMAIL";
pub const COMPILE_OCKAM_CONTROLLER_ADDRESS: &str = "COMPILE_OCKAM_CONTROLLER_ADDRESS";
pub const COMPILE_OCKAM_CONTROLLER_IDENTIFIER: &str = "COMPILE_OCKAM_CONTROLLER_IDENTIFIER";
pub const COMPILE_OCKAM_COMMANDS: &str = "COMPILE_OCKAM_COMMANDS";

// Compile time environment variables names used in the command binary
const COMPILE_DEVELOPER: &str = env!("COMPILE_OCKAM_DEVELOPER");
const COMPILE_HOME: &str = env!("COMPILE_OCKAM_HOME");
const COMPILE_BIN_NAME: &str = env!("COMPILE_OCKAM_COMMAND_BIN_NAME");
const COMPILE_BRAND_NAME: &str = env!("COMPILE_OCKAM_COMMAND_BRAND_NAME");
const COMPILE_SUPPORT_EMAIL: &str = env!("COMPILE_OCKAM_COMMAND_SUPPORT_EMAIL");
const COMPILE_CONTROLLER_ADDRESS: &str = env!("COMPILE_OCKAM_CONTROLLER_ADDRESS");
const COMPILE_CONTROLLER_IDENTIFIER: &str = env!("COMPILE_OCKAM_CONTROLLER_IDENTIFIER");
/// A comma separated list of commands that can be run
/// in the format `command1=customName,command2,command3`
const COMPILE_COMMANDS: &str = env!("COMPILE_OCKAM_COMMANDS");

pub fn get_compile_time_vars() -> Vec<(&'static str, &'static str)> {
    vec![
        (COMPILE_OCKAM_DEVELOPER, COMPILE_DEVELOPER),
        (COMPILE_OCKAM_HOME, COMPILE_HOME),
        (COMPILE_OCKAM_COMMAND_BIN_NAME, COMPILE_BIN_NAME),
        (COMPILE_OCKAM_COMMAND_BRAND_NAME, COMPILE_BRAND_NAME),
        (COMPILE_OCKAM_COMMAND_SUPPORT_EMAIL, COMPILE_SUPPORT_EMAIL),
        (COMPILE_OCKAM_CONTROLLER_ADDRESS, COMPILE_CONTROLLER_ADDRESS),
        (
            COMPILE_OCKAM_CONTROLLER_IDENTIFIER,
            COMPILE_CONTROLLER_IDENTIFIER,
        ),
        (COMPILE_OCKAM_COMMANDS, COMPILE_COMMANDS),
    ]
}

pub fn load_compile_time_vars() {
    // If OCKAM_DEVELOPER is not set, set it to the COMPILE_OCKAM_DEVELOPER value
    if get_env_ignore_error::<bool>(ockam_api::logs::env_variables::OCKAM_DEVELOPER).is_none() {
        std::env::set_var(
            ockam_api::logs::env_variables::OCKAM_DEVELOPER,
            COMPILE_DEVELOPER,
        );
    }
    // Override env vars with compile time values if they are not set
    if let Ok(home_dir) = get_env_with_default(OCKAM_HOME, COMPILE_HOME.to_string()) {
        if !home_dir.is_empty() {
            std::env::set_var(OCKAM_HOME, home_dir);
        }
    }
    if let Ok(orchestrator_identifier) = get_env_with_default(
        OCKAM_CONTROLLER_IDENTIFIER,
        COMPILE_CONTROLLER_IDENTIFIER.to_string(),
    ) {
        if !orchestrator_identifier.is_empty() {
            std::env::set_var(OCKAM_CONTROLLER_IDENTIFIER, orchestrator_identifier);
        }
    }
    if let Ok(orchestrator_address) = get_env_with_default(
        OCKAM_CONTROLLER_ADDRESS,
        COMPILE_CONTROLLER_ADDRESS.to_string(),
    ) {
        if !orchestrator_address.is_empty() {
            std::env::set_var(OCKAM_CONTROLLER_ADDRESS, orchestrator_address);
        }
    }
}

static BRANDING_ENV_VARS: Lazy<BrandingCompileEnvVars> = Lazy::new(|| {
    BrandingCompileEnvVars::new(
        COMPILE_BIN_NAME,
        COMPILE_BRAND_NAME,
        COMPILE_HOME,
        COMPILE_SUPPORT_EMAIL,
        COMPILE_COMMANDS,
        bool::from_string(COMPILE_DEVELOPER).unwrap_or(false),
    )
});

pub struct BrandingCompileEnvVars {
    bin_name: &'static str,
    brand_name: &'static str,
    home_dir: &'static str,
    support_email: &'static str,
    commands: &'static str,
    is_ockam_developer: bool,
}

impl BrandingCompileEnvVars {
    pub fn new(
        bin_name: &'static str,
        brand_name: &'static str,
        home_dir: &'static str,
        support_email: &'static str,
        commands: &'static str,
        is_ockam_developer: bool,
    ) -> Self {
        Self {
            bin_name,
            brand_name,
            home_dir,
            support_email,
            commands,
            is_ockam_developer,
        }
    }

    fn get() -> &'static Self {
        &BRANDING_ENV_VARS
    }

    pub fn bin_name() -> &'static str {
        Self::get().bin_name
    }

    pub fn brand_name() -> &'static str {
        Self::get().brand_name
    }

    pub fn home_dir() -> &'static str {
        Self::get().home_dir
    }

    pub fn support_email() -> &'static str {
        Self::get().support_email
    }

    pub fn commands() -> &'static str {
        Self::get().commands
    }

    pub fn is_ockam_developer() -> bool {
        Self::get().is_ockam_developer
    }
}
