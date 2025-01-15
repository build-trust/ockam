use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Subcommand};
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::orchestrator::subscription::{SubscriptionLegacy, Subscriptions};
use ockam_api::orchestrator::ControllerClient;

use ockam_api::nodes::InMemoryNode;
use ockam_api::output::Output;

use crate::shared_args::IdentityOpts;
use crate::{docs, CommandGlobalOpts, Result};

#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide())]
pub struct SubscriptionCommand {
    #[command(subcommand)]
    subcommand: SubscriptionSubcommand,

    #[command(flatten)]
    identity_opts: IdentityOpts,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SubscriptionSubcommand {
    /// Show the details of a single subscription.
    /// You can use either the subscription ID or the space ID.
    #[command(arg_required_else_help = true)]
    Show {
        /// Subscription ID
        #[arg(group = "id")]
        subscription_id: Option<String>,

        /// Space ID
        #[arg(
            group = "id",
            id = "space",
            value_name = "SPACE_ID",
            long,
            value_parser(NonEmptyStringValueParser::new())
        )]
        space_id: Option<String>,
    },
}

impl SubscriptionCommand {
    pub fn name(&self) -> String {
        match &self.subcommand {
            SubscriptionSubcommand::Show { .. } => "subscription show",
        }
        .to_string()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let controller = node.create_controller().await?;

        match &self.subcommand {
            SubscriptionSubcommand::Show {
                subscription_id,
                space_id,
            } => {
                match get_subscription_by_id_or_space_id(
                    &controller,
                    ctx,
                    subscription_id.clone(),
                    space_id.clone(),
                )
                .await?
                {
                    Some(subscription) => opts.terminal.write_line(&subscription.item()?)?,
                    None => opts
                        .terminal
                        .write_line("Please specify either a space id or a subscription id")?,
                }
            }
        };
        Ok(())
    }
}

pub(crate) async fn get_subscription_by_id_or_space_id(
    controller: &ControllerClient,
    ctx: &Context,
    subscription_id: Option<String>,
    space_id: Option<String>,
) -> Result<Option<SubscriptionLegacy>> {
    match (subscription_id, space_id) {
        (Some(subscription_id), _) => Ok(Some(
            controller
                .get_subscription(ctx, subscription_id.clone())
                .await
                .and_then(|s| s.found())
                .into_diagnostic()?
                .ok_or_else(|| {
                    miette!(
                        "no subscription found for subscription id {}",
                        subscription_id
                    )
                })?,
        )),
        (None, Some(space_id)) => Ok(Some(
            controller
                .get_subscription_by_space_id(ctx, space_id.clone())
                .await
                .and_then(|s| s.found())
                .into_diagnostic()?
                .ok_or_else(|| miette!("no subscription found for space {}", space_id))?,
        )),
        _ => Ok(None),
    }
}
