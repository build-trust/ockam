use crate::docs;
use crate::util::embedded_node;
use crate::Result;
use anyhow::{anyhow, Context as _};
use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Subcommand};
use ockam::compat::collections::HashMap;
use ockam::identity::{AttributesEntry, IdentityIdentifier};
use ockam::{Context, TcpTransport};
use ockam_api::auth;
use ockam_api::is_local_node;
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
        #[arg(long, value_parser(NonEmptyStringValueParser::new()))]
        id: String,
    },

    List {
        /// Address to connect to.
        addr: MultiAddr,
    },
}

impl AuthenticatedCommand {
    pub fn run(self) {
        if let Err(e) = embedded_node(run_impl, self.subcommand) {
            eprintln!("Ockam node failed: {e:?}",);
        }
    }
}

async fn run_impl(ctx: Context, cmd: AuthenticatedSubcommand) -> crate::Result<()> {
    // FIXME: add support to target remote nodes.
    let tcp = TcpTransport::create(&ctx).await?;
    match &cmd {
        AuthenticatedSubcommand::Get { addr, id } => {
            is_local_node(addr).context("The address must point to a local node")?;
            let mut c = client(&ctx, &tcp, addr).await?;
            if let Some(entry) = c.get(id).await? {
                print_entries(&[(IdentityIdentifier::try_from(id.to_string()).unwrap(), entry)]);
            } else {
                println!("Not found");
            }
        }
        AuthenticatedSubcommand::List { addr } => {
            is_local_node(addr).context("The address must point to a local node")?;
            let mut c = client(&ctx, &tcp, addr).await?;
            print_entries(&c.list().await?);
        }
    }

    Ok(())
}

fn print_entries(entries: &[(IdentityIdentifier, AttributesEntry)]) {
    let template = TextTemplate::from(LIST_VIEW);
    let model: Vec<_> = entries
        .iter()
        .map(|(identifier, entry)| {
            let attrs: HashMap<String, String> = entry
                .attrs()
                .iter()
                .map(|(k, v)| (k.to_string(), String::from_utf8(v.clone()).unwrap()))
                .collect();
            (
                String::from(identifier),
                serde_json::to_string(&attrs).unwrap(),
                format!("{:?}", entry.added().unix_time()),
                entry
                    .expires()
                    .map_or("-".to_string(), |t| format!("{:?}", t.unix_time())),
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

async fn client(ctx: &Context, tcp: &TcpTransport, addr: &MultiAddr) -> Result<auth::Client> {
    let route = ockam_api::multiaddr_to_route(addr, tcp)
        .await
        .ok_or_else(|| anyhow!("failed to parse address: {addr}"))?;
    let cl = auth::Client::new(route.route, ctx).await?;
    Ok(cl)
}
