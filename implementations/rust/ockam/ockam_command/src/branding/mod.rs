pub mod command;
pub mod compile_env_vars;

pub use compile_env_vars::{load_compile_time_vars, BrandingCompileEnvVars};
use ockam_api::command::Commands;
use ockam_api::output::OutputBranding;
use once_cell::sync::Lazy;

pub(crate) static OUTPUT_BRANDING: Lazy<OutputBranding> = Lazy::new(|| {
    OutputBranding::new(
        BrandingCompileEnvVars::brand_name().to_string(),
        BrandingCompileEnvVars::bin_name().to_string(),
        Commands::new(BrandingCompileEnvVars::commands()),
    )
});
