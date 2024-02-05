use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;

use ockam::identity::{CredentialSqlxDatabase, Identifier};

use crate::credential::CredentialOutput;
use crate::node::NodeOpts;
use crate::util::async_cmd;
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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |_ctx| async move {
            self.async_run(opts).await
        })
    }

    pub fn name(&self) -> String {
        "list credentials".into()
    }

    async fn async_run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node_name = match self.node_opts.at_node.clone() {
            Some(name) => name,
            None => opts.state.get_default_node().await?.name(),
        };
        let mut opts = opts.clone();
        opts.state.set_node_name(&node_name);

        let database = opts.state.database();
        let storage = CredentialSqlxDatabase::new(database);

        let credentials = storage.get_all().await.into_diagnostic()?;

        let credentials = credentials
            .into_iter()
            .map(|c| CredentialOutput::from_credential(c, true))
            .collect::<Result<Vec<CredentialOutput>>>()?;

        let list = opts.terminal.build_list(
            &credentials,
            "Credentials",
            &format!(
                "No Credentials found for vault: {}",
                node_name.color(OckamColor::PrimaryResource.color())
            ),
        )?;

        opts.terminal.stdout().plain(list).write_line()?;

        Ok(())
    }
}
