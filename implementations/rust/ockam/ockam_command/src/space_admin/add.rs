use crate::shared_args::IdentityOpts;
use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use ockam::Context;
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::email_address::EmailAddress;
use ockam_api::orchestrator::space::Spaces;

/// Add a new Admin to a Space
#[derive(Clone, Debug, Args)]
#[command()]
pub struct AddCommand {
    /// Email of the Admin to add
    #[arg(value_parser = EmailAddress::parse)]
    email: EmailAddress,

    /// Name of the Space
    name: Option<String>,

    #[command(flatten)]
    identity_opts: IdentityOpts,
}

#[async_trait]
impl Command for AddCommand {
    const NAME: &'static str = "space-admin add";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let space = opts.state.get_space_by_name_or_default(&self.name).await?;
        let node =
            InMemoryNode::start_with_identity(ctx, &opts.state, self.identity_opts.identity_name)
                .await?;
        let admin = node
            .add_space_admin(ctx, &space.space_id(), &self.email)
            .await?;

        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "Email {} added as an admin to space {}",
                color_primary(self.email.to_string()),
                color_primary(space.space_name())
            ))
            .json_obj(admin)?
            .write_line()?;
        Ok(())
    }
}
