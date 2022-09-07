use anyhow::{anyhow, Context as _};
use core::fmt::Write;
use std::path::PathBuf;

use clap::{Args, Subcommand};

use ockam::Context;
use ockam_api::cloud::subscription::{ActivateSubscription, Subscription};
use ockam_api::cloud::CloudRequestWrapper;
use ockam_core::api::Request;
use ockam_core::CowStr;
use ockam_multiaddr::MultiAddr;

use crate::node::util::delete_embedded_node;
use crate::util::api::CloudOpts;
use crate::util::output::Output;
use crate::util::{node_rpc, Rpc, RpcBuilder};
use crate::{help, CommandGlobalOpts};

const HELP_DETAIL: &str = "";

#[derive(Clone, Debug, Args)]
#[clap(hide = help::hide(), help_template = help::template(HELP_DETAIL))]
pub struct SubscriptionCommand {
    #[clap(subcommand)]
    subcommand: SubscriptionSubcommand,

    #[clap(flatten)]
    cloud_opts: CloudOpts,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SubscriptionSubcommand {
    /// Attach a subscription to a space
    Attach {
        /// Path to the AWS json file with subscription details
        json: PathBuf,

        /// Space ID to attach the subscription to
        #[clap(
            name = "space",
            value_name = "SPACE_ID",
            long,
            forbid_empty_values = true
        )]
        space_id: String,
    },

    /// Show the details of a single subscription.
    /// You can use either the subscription ID or the space ID.
    Show {
        /// Subscription ID
        #[clap(group = "id")]
        subscription_id: Option<String>,

        /// Space ID
        #[clap(
            group = "id",
            name = "space",
            value_name = "SPACE_ID",
            long,
            forbid_empty_values = true
        )]
        space_id: Option<String>,
    },

    /// Show the details of all subscriptions
    List,

    /// Disable a subscription.
    /// You can use either the subscription ID or the space ID.
    Unsubscribe {
        /// Subscription ID
        #[clap(group = "id")]
        subscription_id: Option<String>,

        /// Space ID
        #[clap(
            group = "id",
            name = "space",
            value_name = "SPACE_ID",
            long,
            forbid_empty_values = true
        )]
        space_id: Option<String>,
    },

    Update(SubscriptionUpdate),
}

#[derive(Clone, Debug, Args)]
pub struct SubscriptionUpdate {
    #[clap(subcommand)]
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
        #[clap(
            group = "id",
            name = "subscription",
            value_name = "SUBSCRIPTION_ID",
            long,
            forbid_empty_values = true
        )]
        subscription_id: Option<String>,

        /// Space ID
        #[clap(
            group = "id",
            name = "space",
            value_name = "SPACE_ID",
            long,
            forbid_empty_values = true
        )]
        space_id: Option<String>,
    },

    /// Move the subscription to a different space.
    /// You can use either the subscription ID or the space ID.
    Space {
        /// Subscription ID
        #[clap(
            group = "id",
            name = "subscription",
            value_name = "SUBSCRIPTION_ID",
            long,
            forbid_empty_values = true
        )]
        subscription_id: Option<String>,

        /// Space ID
        #[clap(
            group = "id",
            name = "current_space",
            value_name = "SPACE_ID",
            long,
            forbid_empty_values = true
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
        SubscriptionSubcommand::Show {
            subscription_id,
            space_id,
        } => {
            let subscription_id = subscription_id_from_cmd_args(
                &ctx,
                &opts,
                rpc.node_name(),
                controller_route,
                subscription_id,
                space_id,
            )
            .await?;
            let req = Request::get(format!("subscription/{}", subscription_id))
                .body(CloudRequestWrapper::bare(controller_route));
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
            let subscription_id = subscription_id_from_cmd_args(
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
                    let subscription_id = subscription_id_from_cmd_args(
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
                    let subscription_id = subscription_id_from_cmd_args(
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

async fn subscription_id_from_cmd_args(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    api_node: &str,
    controller_route: &MultiAddr,
    subscription_id: Option<String>,
    space_id: Option<String>,
) -> crate::Result<String> {
    match (subscription_id, space_id) {
        (_, Some(space_id)) => {
            subscription_id_from_space_id(ctx, opts, api_node, controller_route, &space_id).await
        }
        (Some(subscription_id), _) => Ok(subscription_id),
        _ => unreachable!(),
    }
}

async fn subscription_id_from_space_id(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    api_node: &str,
    controller_route: &MultiAddr,
    space_id: &str,
) -> crate::Result<String> {
    let mut rpc = RpcBuilder::new(ctx, opts, api_node).build();
    let req = Request::get("subscription").body(CloudRequestWrapper::bare(controller_route));
    rpc.request(req).await?;
    let subscriptions = rpc.parse_response::<Vec<Subscription>>()?;
    let subscription = subscriptions
        .into_iter()
        .find(|s| s.space_id == Some(CowStr::from(space_id)))
        .ok_or_else(|| anyhow!("no subscription found for space {}", space_id))?;
    Ok(subscription.id.to_string())
}

impl Output for Subscription<'_> {
    fn output(&self) -> anyhow::Result<String> {
        let mut w = String::new();
        write!(w, "Subscription")?;
        write!(w, "\n  Id: {}", self.id)?;
        write!(w, "\n  Status: {}", self.status)?;
        write!(
            w,
            "\n  Space id: {}",
            self.space_id.as_ref().unwrap_or(&CowStr::from("N/A"))
        )?;
        write!(w, "\n  Entitlements: {}", self.entitlements.as_ref())?;
        write!(w, "\n  Metadata: {}", self.metadata.as_ref())?;
        write!(w, "\n  Contact info: {}", self.contact_info.as_ref())?;
        Ok(w)
    }
}

impl<'a> Output for Vec<Subscription<'a>> {
    fn output(&self) -> anyhow::Result<String> {
        if self.is_empty() {
            return Ok("No subscriptions found".to_string());
        }
        let mut w = String::new();
        for (idx, s) in self.iter().enumerate() {
            write!(w, "\n{idx}:")?;
            write!(w, "\n  Id: {}", s.id)?;
            write!(w, "\n  Status: {}", s.status)?;
            write!(
                w,
                "\n  Space id: {}",
                s.space_id.as_ref().unwrap_or(&CowStr::from("N/A"))
            )?;
            write!(w, "\n  Entitlements: {}", s.entitlements.as_ref())?;
            write!(w, "\n  Metadata: {}", s.metadata.as_ref())?;
            write!(w, "\n  Contact info: {}", s.contact_info.as_ref())?;
            writeln!(w)?;
        }
        Ok(w)
    }
}
