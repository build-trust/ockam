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
use ockam_api::orchestrator::project::ProjectsOrchestratorApi;

/// Add a new Admin to a Project
#[derive(Clone, Debug, Args)]
#[command()]
pub struct AddCommand {
    /// Email of the Admin to add
    #[arg(value_parser = EmailAddress::parse)]
    email: EmailAddress,

    /// Name of the Project
    name: Option<String>,

    #[command(flatten)]
    identity_opts: IdentityOpts,
}

#[async_trait]
impl Command for AddCommand {
    const NAME: &'static str = "project-admin add";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let project = opts
            .state
            .projects()
            .get_project_by_name_or_default(&self.name)
            .await?;
        let node = InMemoryNode::start_with_identity_and_project_name(
            ctx,
            &opts.state,
            self.identity_opts.identity_name,
            Some(project.project_name().to_string()),
        )
        .await?;
        let admin = node
            .add_project_admin(ctx, project.project_id(), &self.email)
            .await?;

        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "Email {} added as an admin to project {}",
                color_primary(self.email.to_string()),
                color_primary(project.project_name())
            ))
            .machine(admin.email.to_string())
            .json_obj(admin)?
            .write_line()?;
        Ok(())
    }
}
