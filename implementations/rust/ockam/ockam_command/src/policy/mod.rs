use clap::{Args, Subcommand};

use ockam::Context;
use ockam_abac::expr::{eq, ident, str};
use ockam_abac::{Action, Resource};
use ockam_api::nodes::models::policy::PolicyList;
use ockam_api::{config::lookup::ProjectLookup, nodes::models::policy::Policy};
use ockam_core::api::Request;

use crate::policy::delete::DeleteCommand;
use crate::policy::list::ListCommand;
use crate::policy::show::ShowCommand;
use crate::{policy::create::CreateCommand, util::Rpc};
use crate::{CommandGlobalOpts, Result};

mod create;
mod delete;
mod list;
mod show;

#[derive(Clone, Debug, Args)]
pub struct PolicyCommand {
    #[command(subcommand)]
    subcommand: PolicySubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum PolicySubcommand {
    #[command(display_order = 900)]
    Create(CreateCommand),
    Show(ShowCommand),
    Delete(DeleteCommand),
    List(ListCommand),
}

impl PolicyCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        match self.subcommand {
            PolicySubcommand::Create(c) => c.run(opts),
            PolicySubcommand::Show(c) => c.run(opts),
            PolicySubcommand::Delete(c) => c.run(opts),
            PolicySubcommand::List(c) => c.run(opts),
        }
    }
}

pub(crate) fn policy_path(r: &Resource, a: &Action) -> String {
    format!("/policy/{r}/{a}")
}

pub(crate) async fn has_policy(
    node: &str,
    ctx: &Context,
    opts: &CommandGlobalOpts,
    resource: &Resource,
) -> Result<bool> {
    let req = Request::get(format!("/policy/{resource}"));
    let mut rpc = Rpc::background(ctx, &opts.state, node).await?;
    let policies: PolicyList = rpc.ask(req).await?;
    Ok(!policies.expressions().is_empty())
}

pub(crate) async fn add_default_project_policy(
    node: &str,
    ctx: &Context,
    opts: &CommandGlobalOpts,
    project: ProjectLookup,
    resource: &Resource,
) -> miette::Result<()> {
    let expr = eq([
        ident("subject.trust_context_id"),
        str(project.id.to_string()),
    ]);
    let bdy = Policy::new(expr);
    let req = Request::post(policy_path(resource, &Action::new("handle_message"))).body(bdy);

    let mut rpc = Rpc::background(ctx, &opts.state, node).await?;
    rpc.tell(req).await?;
    Ok(())
}
