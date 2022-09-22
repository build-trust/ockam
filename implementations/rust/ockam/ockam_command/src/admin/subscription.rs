use anyhow::Context as _;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use clap::builder::{NonEmptyStringValueParser};

use ockam::Context;
use ockam_api::cloud::subscription::{ActivateSubscription, Subscription};
use ockam_api::cloud::CloudRequestWrapper;
use ockam_core::api::Request;

use crate::node::util::delete_embedded_node;
use crate::subscription::utils;
use crate::util::api::CloudOpts;
use crate::util::{node_rpc, Rpc};
use crate::{help, CommandGlobalOpts};

const HELP_DETAIL: &str = "";

#[derive(Clone, Debug, Args)]
#[command(hide = help::hide(), help_template = help::template(HELP_DETAIL))]
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
) -> crate::Result<()> {
    let controller_route = &cmd.cloud_opts.route();
    let mut rpc = Rpc::embedded(&ctx, &opts).await?;
    match cmd.subcommand {
        SubscriptionSubcommand::Attach {
            json,
            space_id: space,
        } => {
            let json =
                std::fs::read_to_string(&json).context(format!("failed to read {:?}", &json))?;
            let b = ActivateSubscription::existing(space, json);
            let req =
                Request::post("subscription").body(CloudRequestWrapper::new(b, controller_route));
            rpc.request(req).await?;
            rpc.parse_and_print_response::<Subscription>()?;
        }
        SubscriptionSubcommand::List => {
            let req =
                Request::get("subscription").body(CloudRequestWrapper::bare(controller_route));
            rpc.request(req).await?;
            rpc.parse_and_print_response::<Vec<Subscription>>()?;
        }
        SubscriptionSubcommand::Unsubscribe {
            subscription_id,
            space_id,
        } => {
            let subscription_id = utils::subscription_id_from_cmd_args(
                &ctx,
                &opts,
                rpc.node_name(),
                controller_route,
                subscription_id,
                space_id,
            )
            .await?;
            let req = Request::put(format!("subscription/{}/unsubscribe", subscription_id))
                .body(CloudRequestWrapper::bare(controller_route));
            rpc.request(req).await?;
            rpc.parse_and_print_response::<Subscription>()?;
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
                        .context(format!("failed to read {:?}", &json))?;
                    let subscription_id = utils::subscription_id_from_cmd_args(
                        &ctx,
                        &opts,
                        rpc.node_name(),
                        controller_route,
                        subscription_id,
                        space_id,
                    )
                    .await?;
                    let req =
                        Request::put(format!("subscription/{}/contact_info", subscription_id))
                            .body(CloudRequestWrapper::new(json, controller_route));
                    rpc.request(req).await?;
                    rpc.parse_and_print_response::<Subscription>()?;
                }
                SubscriptionUpdateSubcommand::Space {
                    subscription_id,
                    space_id,
                    new_space_id,
                } => {
                    let subscription_id = utils::subscription_id_from_cmd_args(
                        &ctx,
                        &opts,
                        rpc.node_name(),
                        controller_route,
                        subscription_id,
                        space_id,
                    )
                    .await?;
                    let req = Request::put(format!("subscription/{}/space_id", subscription_id))
                        .body(CloudRequestWrapper::new(new_space_id, controller_route));
                    rpc.request(req).await?;
                    rpc.parse_and_print_response::<Subscription>()?;
                }
            }
        }
    };
    delete_embedded_node(&opts.config, rpc.node_name()).await;
    Ok(())
}
