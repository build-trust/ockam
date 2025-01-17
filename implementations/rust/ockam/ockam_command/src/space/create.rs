use async_trait::async_trait;
use clap::Args;
use miette::miette;

use ockam::Context;
use ockam_api::cli_state::random_name;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::space::Spaces;
use ockam_api::output::Output;

use crate::shared_args::IdentityOpts;
use crate::util::validators::cloud_resource_name_validator;
use crate::{docs, Command, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a new space
#[derive(Clone, Debug, Args)]
#[command(
hide = true,
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct CreateCommand {
    #[arg(display_order = 1001, value_name = "SPACE_NAME", default_value_t = random_name(), hide_default_value = true, value_parser = validate_space_name)]
    #[arg(help = docs::about("Name of the `BIN_NAME` space - must be unique across all Ockam Orchestrator users."))]
    pub name: String,

    /// Administrators for this space
    #[arg(display_order = 1100, last = true)]
    pub admins: Vec<String>,

    #[command(flatten)]
    pub identity_opts: IdentityOpts,
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "space create";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        if !opts
            .state
            .is_identity_enrolled(&self.identity_opts.identity_name)
            .await?
        {
            return Err(miette!(
                "Please enroll using 'ockam enroll' before using this command"
            ));
        };

        let node = InMemoryNode::start(ctx, &opts.state).await?;

        let space = {
            let pb = opts.terminal.spinner();
            if let Some(pb) = pb.as_ref() {
                pb.set_message("Creating a Space for you...");
            }
            node.create_space(
                ctx,
                &self.name,
                self.admins.iter().map(|a| a.as_ref()).collect(),
            )
            .await?
        };
        if let Ok(msg) = space.subscription_status_message() {
            opts.terminal.write_line(msg)?;
        }
        opts.terminal
            .stdout()
            .plain(space.item()?)
            .json_obj(&space)?
            .write_line()?;
        Ok(())
    }
}

fn validate_space_name(s: &str) -> std::result::Result<String, String> {
    match cloud_resource_name_validator(s) {
        Ok(_) => Ok(s.to_string()),
        Err(_e) => Err(String::from(
            "The Space name can contain only alphanumeric characters and the '-', '_' and '.' separators. \
            Separators must occur between alphanumeric characters. This implies that separators can't \
            occur at the start or end of the name, nor they can occur in sequence.",
        ))
    }
}
