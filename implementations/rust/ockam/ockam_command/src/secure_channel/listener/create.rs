use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::nodes::models::secure_channel::CreateSecureChannelListenerRequest;
use ockam_api::nodes::{BackgroundNode, NODEMANAGER_ADDR};
use ockam_core::api::{Request, Status};
use ockam_core::{Address, Route};

use crate::node::util::initialize_default_node;
use crate::node::NodeOpts;
use crate::util::{api, exitcode, node_rpc};
use crate::{docs, fmt_log, fmt_ok, terminal::OckamColor, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create Secure Channel Listeners
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Address for this listener
    address: Address,

    /// Authorized Identifiers of secure channel initiators
    #[arg(short, long, value_name = "IDENTIFIERS")]
    authorized: Option<Vec<Identifier>>,

    /// Name of the Vault that the secure-channel listener will use
    #[arg(value_name = "VAULT_NAME", long, requires = "identity")]
    vault: Option<String>,

    /// Name of the Identity that the secure-channel listener will use
    #[arg(value_name = "IDENTITY_NAME", long)]
    identity: Option<String>,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> miette::Result<()> {
    run_impl(&ctx, (opts, cmd)).await
}

async fn run_impl(
    ctx: &Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    initialize_default_node(ctx, &opts).await?;
    let node = BackgroundNode::create(ctx, &opts.state, &cmd.node_opts.at_node).await?;
    let req = Request::post("/node/secure_channel_listener").body(
        CreateSecureChannelListenerRequest::new(
            &cmd.address,
            cmd.authorized,
            cmd.vault,
            cmd.identity,
        ),
    );
    let result = node.tell(ctx, req).await;
    match result {
        Ok(_) => {
            let address = format!("/service/{}", cmd.address.address());
            opts.terminal
                .stdout()
                .plain(
                    fmt_ok!(
                        "Secure Channel Listener at {} created successfully\n",
                        address
                            .to_string()
                            .color(OckamColor::PrimaryResource.color())
                    ) + &fmt_log!(
                        "At node /node/{}",
                        node.node_name().color(OckamColor::PrimaryResource.color())
                    ),
                )
                .machine(address.to_string())
                .json(serde_json::json!([{ "address": address }]))
                .write_line()?;
            Ok(())
        }
        Err(e) => Err(miette!(
            "An error occurred while creating the secure channel listener"
        ))
        .context(e),
    }
}

pub async fn create_listener(
    ctx: &Context,
    addr: Address,
    authorized_identifiers: Option<Vec<Identifier>>,
    identity: Option<String>,
    mut base_route: Route,
) -> miette::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::create_secure_channel_listener(&addr, authorized_identifiers, identity)?,
        )
        .await
        .into_diagnostic()?;

    let response = api::parse_create_secure_channel_listener_response(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            println!("/service/{}", addr.address());
            Ok(())
        }
        _ => {
            eprintln!("An error occurred while creating the secure channel listener",);
            std::process::exit(exitcode::CANTCREAT)
        }
    }
}
