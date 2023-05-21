use core::fmt::Write;

use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Subcommand};

use ockam::Context;
use ockam_api::cloud::subscription::Subscription;
use ockam_api::cloud::CloudRequestWrapper;
use ockam_core::api::Request;
use ockam_core::CowStr;

use crate::node::util::delete_embedded_node;
use crate::util::api::CloudOpts;
use crate::util::output::Output;
use crate::util::{node_rpc, Rpc};
use crate::{docs, CommandGlobalOpts, Result};

#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide())]
pub struct SubscriptionCommand {
    #[command(subcommand)]
    subcommand: SubscriptionSubcommand,

    #[command(flatten)]
    cloud_opts: CloudOpts,
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
        SubscriptionSubcommand::Show {
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
            let req = Request::get(format!("subscription/{subscription_id}"))
                .body(CloudRequestWrapper::bare(controller_route));
            rpc.request(req).await?;
            rpc.parse_and_print_response::<Subscription>()?;
        }
    };
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}

pub mod utils {
    use miette::miette;

    use ockam_multiaddr::MultiAddr;

    use crate::util::RpcBuilder;

    use super::*;

    pub async fn subscription_id_from_cmd_args(
        ctx: &Context,
        opts: &CommandGlobalOpts,
        api_node: &str,
        controller_route: &MultiAddr,
        subscription_id: Option<String>,
        space_id: Option<String>,
    ) -> crate::Result<String> {
        match (subscription_id, space_id) {
            (_, Some(space_id)) => {
                subscription_id_from_space_id(ctx, opts, api_node, controller_route, &space_id)
                    .await
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
            .ok_or_else(|| miette!("no subscription found for space {}", space_id))?;
        Ok(subscription.id.to_string())
    }
}

impl Output for Subscription<'_> {
    fn output(&self) -> Result<String> {
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

impl Output for Vec<Subscription<'_>> {
    fn output(&self) -> Result<String> {
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
