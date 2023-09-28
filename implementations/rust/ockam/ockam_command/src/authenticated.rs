use crate::node::get_node_name;
use crate::util::node_rpc;
use crate::util::parsers::identity_identifier_parser;
use crate::Result;
use crate::{docs, CommandGlobalOpts};
use clap::{Args, Subcommand};
use miette::Context as _;
use ockam::compat::collections::HashMap;
use ockam::identity::{AttributesEntry, Identifier};
use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::auth::AuthorizationApi;
use ockam_api::is_local_node;
use ockam_api::nodes::BackgroundNode;
use ockam_multiaddr::MultiAddr;
use termimad::{minimad::TextTemplate, MadSkin};

const HELP_DETAIL: &str = "";

const LIST_VIEW: &str = r#"
## Authenticated Identities

${identity
> **Identifier:** ${identifier}
> **Attributes:** ${attributes}
> **Added At:** ${created_at}
> **Expires At:** ${expires_at}
> **Attested By:** ${attested_by}

}
"#;

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
    match &cmd {
        AuthenticatedSubcommand::Get { addr, id } => {
            let node = make_background_node_client(&ctx, &opts, addr).await?;
            if let Some(entry) = node.get_attributes(&ctx, id).await? {
                print_entries(&[(Identifier::try_from(id.to_string()).unwrap(), entry)]);
            } else {
                println!("Not found");
            }
        }
        AuthenticatedSubcommand::List { addr } => {
            let node = make_background_node_client(&ctx, &opts, addr).await?;
            let entries = node.list_identifiers(&ctx).await?;
            print_entries(&entries);
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

fn print_entries(entries: &[(Identifier, AttributesEntry)]) {
    let template = TextTemplate::from(LIST_VIEW);
    let model: Vec<_> = entries
        .iter()
        .map(|(identifier, entry)| {
            let attrs: HashMap<String, String> = entry
                .attrs()
                .iter()
                .map(|(k, v)| {
                    (
                        String::from_utf8(k.clone()).unwrap(),
                        String::from_utf8(v.clone()).unwrap(),
                    )
                })
                .collect();
            (
                String::from(identifier),
                serde_json::to_string(&attrs).unwrap(),
                format!("{:?}", entry.added()),
                entry
                    .expires()
                    .map_or("-".to_string(), |t| format!("{:?}", t)),
                entry.attested_by().map_or("-".to_string(), String::from),
            )
        })
        .collect();
    let mut expander = template.expander();

    model.iter().for_each(
        |(identifier, attributes, created_at, expires_at, attested_by)| {
            expander
                .sub("identity")
                .set("identifier", identifier)
                .set("attributes", attributes)
                .set("created_at", created_at)
                .set("expires_at", expires_at)
                .set("attested_by", attested_by);
        },
    );
    let skin = MadSkin::default();
    skin.print_expander(expander);
}
