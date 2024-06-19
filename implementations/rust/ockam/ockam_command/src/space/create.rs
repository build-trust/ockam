use clap::Args;
use colorful::Colorful;
use miette::miette;

use ockam::Context;
use ockam_api::cli_state::random_name;
use ockam_api::cloud::space::Spaces;
use ockam_api::nodes::InMemoryNode;

use crate::shared_args::IdentityOpts;
use crate::util::async_cmd;
use crate::util::validators::cloud_resource_name_validator;
use crate::{docs, CommandGlobalOpts};
use ockam_api::output::Output;

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a new space
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// Name of the space - must be unique across all Ockam Orchestrator users.
    #[arg(display_order = 1001, value_name = "SPACE_NAME", default_value_t = random_name(), hide_default_value = true, value_parser = validate_space_name)]
    pub name: String,

    /// Administrators for this space
    #[arg(display_order = 1100, last = true)]
    pub admins: Vec<String>,

    #[command(flatten)]
    pub identity_opts: IdentityOpts,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "space crate".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        if !opts
            .state
            .is_identity_enrolled(&self.identity_opts.identity_name)
            .await?
        {
            return Err(miette!(
                "Please enroll using 'ockam enroll' before using this command"
            ));
        };

        opts.terminal.write_line(format!(
            "\n{}",
            "Creating a trial space for you (everything in it will be deleted in 15 days) ..."
                .light_magenta(),
        ))?;
        opts.terminal.write_line(format!(
            "{}",
            "To learn more about production ready spaces in Ockam Orchestrator, contact us at: hello@ockam.io".light_magenta()
        ))?;

        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let space = node
            .create_space(
                ctx,
                &self.name,
                self.admins.iter().map(|a| a.as_ref()).collect(),
            )
            .await?;

        opts.terminal
            .stdout()
            .plain(space.item()?)
            .json(serde_json::json!(&space))
            .write_line()?;
        Ok(())
    }
}

fn validate_space_name(s: &str) -> Result<String, String> {
    match cloud_resource_name_validator(s) {
        Ok(_) => Ok(s.to_string()),
        Err(_e) => Err(String::from(
            "space name can contain only alphanumeric characters and the '-', '_' and '.' separators. \
            Separators must occur between alphanumeric characters. This implies that separators can't \
            occur at the start or end of the name, nor they can occur in sequence.",
        ))
    }
}
