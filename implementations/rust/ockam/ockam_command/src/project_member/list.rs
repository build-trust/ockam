use async_trait::async_trait;
use clap::Args;

use ockam::Context;
use ockam_api::authenticator::direct::{
    Members, OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE, OCKAM_ROLE_ATTRIBUTE_KEY,
};

use crate::shared_args::IdentityOpts;
use crate::{docs, Command, CommandGlobalOpts, Result};

use super::{authority_client, MemberOutput};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");

/// List members of a Project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
)]
pub struct ListCommand {
    #[command(flatten)]
    identity_opts: IdentityOpts,

    /// The Project to list members from
    #[arg(long, short, value_name = "PROJECT_NAME")]
    project_name: Option<String>,

    /// Return only the enroller members
    #[arg(long, visible_alias = "enroller")]
    enrollers: bool,
}

#[async_trait]
impl Command for ListCommand {
    const NAME: &'static str = "project-member list";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let (authority_node_client, _) =
            authority_client(ctx, &opts, &self.identity_opts, &self.project_name).await?;

        let members = authority_node_client
            .list_members(ctx)
            .await?
            .into_iter()
            .filter(|(_, a)| {
                !self.enrollers
                    || a.deserialized_key_value_attrs().contains(&format!(
                        "{}={}",
                        OCKAM_ROLE_ATTRIBUTE_KEY, OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE
                    ))
            })
            .map(|(i, a)| MemberOutput::new(i, a))
            .collect::<Vec<_>>();

        let plain = opts
            .terminal
            .build_list(&members, "No members found on the Authority node")?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json_obj(&members)?
            .write_line()?;

        Ok(())
    }
}
