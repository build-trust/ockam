use async_trait::async_trait;
use clap::Args;
use serde::Serialize;

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::authenticator::direct::Members;
use ockam_multiaddr::MultiAddr;

use super::authority_client;
use crate::shared_args::IdentityOpts;
use crate::{docs, Command, CommandGlobalOpts, Result};
use ockam_api::output::Output;

const LONG_ABOUT: &str = include_str!("./static/list_ids/long_about.txt");

/// List members ID's of a Project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
)]
pub struct ListIdsCommand {
    #[command(flatten)]
    identity_opts: IdentityOpts,

    /// The route to the Project to list members from
    #[arg(long, short, value_name = "ROUTE_TO_PROJECT")]
    to: Option<MultiAddr>,
}

#[async_trait]
impl Command for ListIdsCommand {
    const NAME: &'static str = "project-member list-ids";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let (authority_node_client, _) =
            authority_client(ctx, &opts, &self.identity_opts, &self.to).await?;

        let member_ids = authority_node_client
            .list_member_ids(ctx)
            .await?
            .into_iter()
            .map(|identifier| ListIdsOutput { identifier })
            .collect::<Vec<_>>();

        let plain = opts
            .terminal
            .build_list(&member_ids, "No members found on the Authority node")?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json_obj(&member_ids)?
            .write_line()?;

        Ok(())
    }
}

#[derive(Serialize)]
struct ListIdsOutput {
    identifier: Identifier,
}

impl Output for ListIdsOutput {
    fn item(&self) -> ockam_api::Result<String> {
        Ok(format!("{}", self.identifier))
    }
}
