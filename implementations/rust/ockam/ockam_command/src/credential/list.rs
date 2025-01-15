use clap::Args;
use miette::IntoDiagnostic;

use crate::credential::LocalCredentialOutput;
use crate::node::NodeOpts;
use crate::util::parsers::identity_identifier_parser;
use crate::CommandGlobalOpts;
use crate::Result;
use ockam::identity::{CredentialSqlxDatabase, Identifier};
use ockam_api::colors::color_primary;

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
    pub fn name(&self) -> String {
        "credential list".into()
    }

    pub async fn run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node_name = match self.node_opts.at_node.clone() {
            Some(name) => name,
            None => opts.state.get_default_node().await?.name(),
        };
        let database = opts.state.database();
        let storage = CredentialSqlxDatabase::new(database, &node_name);

        let credentials = storage.get_all().await.into_diagnostic()?;

        let credentials = credentials
            .into_iter()
            .map(|c| LocalCredentialOutput::from_credential(c.0, c.1, true))
            .collect::<Result<Vec<LocalCredentialOutput>>>()?;

        let list = opts.terminal.build_list(
            &credentials,
            &format!(
                "No Credentials found for node: {}",
                color_primary(node_name)
            ),
        )?;

        opts.terminal
            .stdout()
            .plain(list)
            .json_obj(credentials)?
            .write_line()?;

        Ok(())
    }
}
