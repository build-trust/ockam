use std::path::PathBuf;

use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Subcommand};
use miette::{Context as _, IntoDiagnostic};

use ockam::Context;
use ockam_api::cloud::subscription::Subscriptions;

use ockam_api::nodes::InMemoryNode;

use crate::output::Output;
use crate::subscription::get_subscription_by_id_or_space_id;
use crate::util::api::CloudOpts;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const HELP_DETAIL: &str = "";

#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide(), after_long_help = docs::after_help(HELP_DETAIL))]
pub struct SubscriptionCommand {
    #[command(subcommand)]
    subcommand: SubscriptionSubcommand,

    #[command(flatten)]
    cloud_opts: CloudOpts,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SubscriptionSubcommand {
    /// Attach a subscription to a space
    Attach {
        /// Path to the AWS json file with subscription details
        json: PathBuf,

        /// Space ID to attach the subscription to
        #[arg(
            id = "space",
            value_name = "SPACE_ID",
            long,
            value_parser(NonEmptyStringValueParser::new())
        )]
        space_id: String,
    },

    /// Show the details of all subscriptions
    List,

    /// Disable a subscription.
    /// You can use either the subscription ID or the space ID.
    Unsubscribe {
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

    /// Update a subscription
    Update(SubscriptionUpdate),
}

#[derive(Clone, Debug, Args)]
pub struct SubscriptionUpdate {
    #[command(subcommand)]
    subcommand: SubscriptionUpdateSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
enum SubscriptionUpdateSubcommand {
    /// Update the contact info of a subscription.
    /// You can use either the subscription ID or the space ID.
    ContactInfo {
        /// Path to the AWS json file with contact info details
        json: PathBuf,

        /// Subscription ID
        #[arg(
            group = "id",
            id = "subscription",
            value_name = "SUBSCRIPTION_ID",
            long,
            value_parser(NonEmptyStringValueParser::new())
        )]
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

    /// Move the subscription to a different space.
    /// You can use either the subscription ID or the space ID.
    Space {
        /// Subscription ID
        #[arg(
            group = "id",
            id = "subscription",
            value_name = "SUBSCRIPTION_ID",
            long,
            value_parser(NonEmptyStringValueParser::new())
        )]
        subscription_id: Option<String>,

        /// Space ID
        #[arg(
            group = "id",
            id = "current_space",
            value_name = "SPACE_ID",
            long,
            value_parser(NonEmptyStringValueParser::new())
        )]
        space_id: Option<String>,

        /// Space ID to move subscription to
        new_space_id: String,
    },
}

impl SubscriptionCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, SubscriptionCommand),
) -> miette::Result<()> {
    let node = InMemoryNode::start(&ctx, &opts.state).await?;
    let controller = node.create_controller().await?;

    match cmd.subcommand {
        SubscriptionSubcommand::Attach {
            json,
            space_id: space,
        } => {
            let json = std::fs::read_to_string(&json)
                .into_diagnostic()
                .context(format!("failed to read {:?}", &json))?;

            let response = controller
                .activate_subscription(&ctx, space, json)
                .await
                .into_diagnostic()?;
            opts.terminal.write_line(&response.output()?)?
        }
        SubscriptionSubcommand::List => {
            let response = controller.get_subscriptions(&ctx).await.into_diagnostic()?;
            opts.terminal.write_line(&response.output()?)?
        }
        SubscriptionSubcommand::Unsubscribe {
            subscription_id,
            space_id,
        } => {
            match get_subscription_by_id_or_space_id(&controller, &ctx, subscription_id, space_id)
                .await?
            {
                Some(subscription) => {
                    let response = controller
                        .unsubscribe(&ctx, subscription.id)
                        .await
                        .into_diagnostic()?;
                    opts.terminal.write_line(&response.output()?)?
                }
                None => opts
                    .terminal
                    .write_line("Please specify either a space id or a subscription id")?,
            }
        }
        SubscriptionSubcommand::Update(c) => {
            let SubscriptionUpdate { subcommand: sc } = c;
            match sc {
                SubscriptionUpdateSubcommand::ContactInfo {
                    json,
                    space_id,
                    subscription_id,
                } => {
                    let json = std::fs::read_to_string(&json)
                        .into_diagnostic()
                        .context(format!("failed to read {:?}", &json))?;
                    match get_subscription_by_id_or_space_id(
                        &controller,
                        &ctx,
                        subscription_id,
                        space_id,
                    )
                    .await?
                    {
                        Some(subscription) => {
                            let response = controller
                                .update_subscription_contact_info(&ctx, subscription.id, json)
                                .await
                                .into_diagnostic()?;
                            opts.terminal.write_line(&response.output()?)?
                        }
                        None => opts
                            .terminal
                            .write_line("Please specify either a space id or a subscription id")?,
                    }
                }
                SubscriptionUpdateSubcommand::Space {
                    subscription_id,
                    space_id,
                    new_space_id,
                } => {
                    match get_subscription_by_id_or_space_id(
                        &controller,
                        &ctx,
                        subscription_id,
                        space_id,
                    )
                    .await?
                    {
                        Some(subscription) => {
                            let response = controller
                                .update_subscription_space(&ctx, subscription.id, new_space_id)
                                .await
                                .into_diagnostic()?;
                            opts.terminal.write_line(&response.output()?)?
                        }
                        None => opts
                            .terminal
                            .write_line("Please specify either a space id or a subscription id")?,
                    }
                }
            }
        }
    };
    Ok(())
}
