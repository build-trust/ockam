use crate::node::get_node_name;
use crate::output::Output;
use crate::util::node_rpc;
use crate::util::parsers::identity_identifier_parser;
use crate::Result;
use crate::{docs, CommandGlobalOpts};
use clap::{Args, Subcommand};
use miette::{miette, Context as _};
use ockam::identity::{AttributesEntry, Identifier};
use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::auth::AuthorizationApi;
use ockam_api::is_local_node;
use ockam_api::nodes::BackgroundNode;
use ockam_multiaddr::MultiAddr;
use std::fmt::Write;

const HELP_DETAIL: &str = "";

#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide(), after_long_help = docs::after_help(HELP_DETAIL))]
pub struct AuthenticatedCommand {
    #[command(subcommand)]
    subcommand: AuthenticatedSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum AuthenticatedSubcommand {
    /// Get attribute value.
    Get {
        /// Address to connect to.
        addr: MultiAddr,

        /// Subject identifier
        #[arg(long, value_name = "IDENTIFIER", value_parser = identity_identifier_parser)]
        id: Identifier,
    },

    List {
        /// Address to connect to.
        addr: MultiAddr,
    },
}

impl AuthenticatedCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self.subcommand))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AuthenticatedSubcommand),
) -> miette::Result<()> {
    // FIXME: add support to target remote nodes.
    match cmd {
        AuthenticatedSubcommand::Get { addr, id } => {
            let node = make_background_node_client(&ctx, &opts, &addr).await?;
            if let Some(entry) = node.get_attributes(&ctx, &id).await? {
                let entries = vec![(id, entry)];
                print_entries(opts, entries)?;
            } else {
                Err(miette!("No attributes found for the given identifier"))?;
            }
        }
        AuthenticatedSubcommand::List { addr } => {
            let node = make_background_node_client(&ctx, &opts, &addr).await?;
            let entries = node.list_identifiers(&ctx).await?;
            print_entries(opts, entries)?;
        }
    }
    Ok(())
}

async fn make_background_node_client(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    addr: &MultiAddr,
) -> Result<BackgroundNode> {
    is_local_node(addr).context("The address must point to a local node")?;
    let to = get_node_name(&opts.state, &Some(addr.to_string()));
    let node_name = extract_address_value(&to)?;
    Ok(BackgroundNode::create(ctx, &opts.state, &node_name).await?)
}

struct IdentifierWithAttributes {
    identifier: Identifier,
    entry: AttributesEntry,
}

impl Output for IdentifierWithAttributes {
    fn output(&self) -> Result<String> {
        let mut output = String::new();
        let attrs: Vec<String> = self
            .entry
            .attrs()
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}: {}",
                    String::from_utf8_lossy(k),
                    String::from_utf8_lossy(v)
                )
            })
            .collect();
        let attrs_str = attrs.join(", ");
        writeln!(output, "Identifier: {}", self.identifier)?;
        writeln!(output, "Attributes: {}", attrs_str)?;
        writeln!(output, "Added At: {:?}", self.entry.added())?;
        writeln!(output, "Expires At: {:?}", self.entry.expires())?;
        write!(
            output,
            "Attested By: {}",
            self.entry
                .attested_by()
                .map(|i| i.to_string())
                .unwrap_or("-".to_string())
        )?;

        Ok(output)
    }
}

fn print_entries(
    opts: CommandGlobalOpts,
    entries: Vec<(Identifier, AttributesEntry)>,
) -> Result<()> {
    let entries = entries
        .into_iter()
        .map(|(identifier, entry)| IdentifierWithAttributes { identifier, entry })
        .collect::<Vec<_>>();
    let list =
        opts.terminal
            .build_list(&entries, "Attributes by identifier", "No attributes found")?;
    opts.terminal.stdout().plain(list).write_line()?;
    Ok(())
}
