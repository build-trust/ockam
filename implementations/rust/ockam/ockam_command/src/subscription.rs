use core::fmt::Write;

use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Subcommand};

use ockam::Context;
use ockam_api::cloud::subscription::Subscription;
use ockam_core::api::Request;

use crate::node::util::delete_embedded_node;
use crate::output::Output;
use crate::util::api::CloudOpts;
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
) -> miette::Result<()> {
    let mut rpc = Rpc::embedded(&ctx, &opts).await?;
    match cmd.subcommand {
        SubscriptionSubcommand::Show {
            subscription_id,
            space_id,
        } => {
            let subscription_id =
                utils::subscription_id_from_cmd_args(&mut rpc, subscription_id, space_id).await?;
            let req = Request::get(format!("subscription/{subscription_id}"));
            let subscription: Subscription = rpc.ask(req).await?;
            opts.println(&subscription)?;
        }
    };
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}

pub mod utils {
    use miette::miette;

    use super::*;

    pub async fn subscription_id_from_cmd_args<'a>(
        rpc: &mut Rpc,
        subscription_id: Option<String>,
        space_id: Option<String>,
    ) -> crate::Result<String> {
        match (subscription_id, space_id) {
            (_, Some(space_id)) => subscription_id_from_space_id(rpc, &space_id).await,
            (Some(subscription_id), _) => Ok(subscription_id),
            _ => unreachable!(),
        }
    }

    async fn subscription_id_from_space_id<'a>(
        rpc: &mut Rpc,
        space_id: &str,
    ) -> crate::Result<String> {
        let req = Request::get("subscription");
        let subscriptions: Vec<Subscription> = rpc.ask(req).await?;
        let subscription = subscriptions
            .into_iter()
            .find(|s| s.space_id == Some(space_id.into()))
            .ok_or_else(|| miette!("no subscription found for space {}", space_id))?;
        Ok(subscription.id)
    }
}

impl Output for Subscription {
    fn output(&self) -> Result<String> {
        let mut w = String::new();
        write!(w, "Subscription")?;
        write!(w, "\n  Id: {}", self.id)?;
        write!(w, "\n  Status: {}", self.status)?;
        write!(
            w,
            "\n  Space id: {}",
            self.space_id.clone().unwrap_or("N/A".to_string())
        )?;
        write!(w, "\n  Entitlements: {}", self.entitlements)?;
        write!(w, "\n  Metadata: {}", self.metadata)?;
        write!(w, "\n  Contact info: {}", self.contact_info)?;
        Ok(w)
    }
}

impl Output for Vec<Subscription> {
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
                s.space_id.as_ref().unwrap_or(&"N/A".to_string())
            )?;
            write!(w, "\n  Entitlements: {}", s.entitlements)?;
            write!(w, "\n  Metadata: {}", s.metadata)?;
            write!(w, "\n  Contact info: {}", s.contact_info)?;
            writeln!(w)?;
        }
        Ok(w)
    }
}
