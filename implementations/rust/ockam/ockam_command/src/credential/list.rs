use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;

use ockam::identity::{CredentialSqlxDatabase, Identifier};
use ockam::Context;

use crate::credential::CredentialOutput;
use crate::node::NodeOpts;
use crate::util::node_rpc;
use crate::util::parsers::identity_identifier_parser;
use crate::Result;
use crate::{CommandGlobalOpts, OckamColor};

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    /// Subject Identifier
    #[arg(long, value_name = "SUBJECT", value_parser = identity_identifier_parser)]
    subject: Option<Identifier>,

    /// Issuer Identifier
    #[arg(long, value_name = "ISSUER", value_parser = identity_identifier_parser)]
    issuer: Option<Identifier>,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(opts.rt.clone(), rpc, (opts, self));
    }
}

async fn rpc(_ctx: Context, (opts, cmd): (CommandGlobalOpts, ListCommand)) -> miette::Result<()> {
    run_impl(opts, cmd).await
}

async fn run_impl(mut opts: CommandGlobalOpts, cmd: ListCommand) -> miette::Result<()> {
    let node_name = match cmd.node_opts.at_node.clone() {
        Some(name) => name,
        None => opts.state.get_default_node().await?.name(),
    };
    opts.state.set_node_name(node_name.clone());

    let database = opts.state.database();
    let storage = CredentialSqlxDatabase::new(database);

    let credentials = storage.get_all().await.into_diagnostic()?;

    let credentials = credentials
        .into_iter()
        .map(|c| CredentialOutput::from_credential(c, true))
        .collect::<Result<Vec<CredentialOutput>>>()?;

    let list = opts.terminal.build_list(
        &credentials,
        &format!("Credentials on {}", node_name),
        &format!(
            "No Credentials found on {}",
            node_name.color(OckamColor::PrimaryResource.color())
        ),
    )?;

    opts.terminal.stdout().plain(list).write_line()?;

    Ok(())
}
