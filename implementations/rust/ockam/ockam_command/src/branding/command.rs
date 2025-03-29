use ockam_api::command::Commands;
use once_cell::sync::Lazy;

pub(crate) fn name(name: &'static str) -> &'static str {
    CUSTOM_COMMANDS.name(name)
}

pub(crate) fn hide(name: &'static str) -> bool {
    CUSTOM_COMMANDS.hide(name)
}

pub(crate) static CUSTOM_COMMANDS: Lazy<Commands> =
    Lazy::new(|| Commands::new(super::BrandingCompileEnvVars::commands()));
