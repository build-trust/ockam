use crate::{docs, pager};
use clap::Args;
use ockam_api::colors::{color_primary, color_warn};
use ockam_api::fmt_log;
use ockam_api::output::Output;
use ockam_api::terminal::{INDENTATION, PADDING};

const ENV_INFO: &str = include_str!("./static/env_info.txt");

#[derive(Clone, Debug, Args)]
#[command(about = docs::about("Outputs information about environment variables used by the Ockam CLI"))]
pub struct EnvironmentCommand {}

impl EnvironmentCommand {
    pub fn run(self) -> miette::Result<()> {
        pager::render_output(&format!("{}\n\n{}", Self::info(), Self::values()));
        Ok(())
    }

    pub fn name(&self) -> String {
        "environment".to_string()
    }

    fn info() -> String {
        ENV_INFO.padded_display()
    }

    fn values() -> String {
        // get all the env vars and filter those containing OCKAM in their name
        let runtime_vars = std::env::vars()
            .filter(|(k, _)| k.contains("OCKAM"))
            .map(|(k, v)| {
                format!(
                    "{}{}{}={}",
                    PADDING,
                    INDENTATION,
                    color_primary(k),
                    color_warn(v)
                )
            })
            .collect::<Vec<String>>()
            .join("\n");
        let compile_vars = crate::branding::compile_env_vars::get_compile_time_vars()
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}{}{}={}",
                    PADDING,
                    INDENTATION,
                    color_primary(k),
                    color_warn(v)
                )
            })
            .collect::<Vec<String>>()
            .join("\n");
        fmt_log!("Values:\n{}\n{}", runtime_vars, compile_vars)
    }
}
