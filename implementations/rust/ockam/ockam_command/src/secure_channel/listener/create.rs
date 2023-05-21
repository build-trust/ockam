use clap::Args;
use miette::miette;

use ockam::identity::IdentityIdentifier;
use ockam::Context;
use ockam_api::nodes::models::secure_channel::CreateSecureChannelListenerRequest;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{Request, Status};
use ockam_core::{Address, Route};

use crate::node::{get_node_name, initialize_node_if_default};
use crate::secure_channel::listener::utils::SecureChannelListenerNodeOpts;
use crate::util::{api, exitcode, extract_address_value, node_rpc, Rpc};
use crate::{docs, CommandGlobalOpts, Result};

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
    node_opts: SecureChannelListenerNodeOpts,

    /// Address for this listener
    address: Address,

    /// Authorized Identifiers of secure channel initiators
    #[arg(short, long, value_name = "IDENTIFIERS")]
    authorized: Option<Vec<IdentityIdentifier>>,

    #[arg(value_name = "VAULT", long, requires = "identity")]
    vault: Option<String>,

    #[arg(value_name = "IDENTITY", long)]
    identity: Option<String>,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at);
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> crate::Result<()> {
    run_impl(&ctx, (opts, cmd)).await
}

async fn run_impl(
    ctx: &Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let at = get_node_name(&opts.state, &cmd.node_opts.at);
    let node = extract_address_value(&at)?;
    let mut rpc = Rpc::background(ctx, &opts, &node)?;
    let req = Request::post("/node/secure_channel_listener").body(
        CreateSecureChannelListenerRequest::new(
            &cmd.address,
            cmd.authorized,
            cmd.vault,
            cmd.identity,
        ),
    );
    rpc.request(req).await?;
    match rpc.is_ok() {
        Ok(_) => {
            println!("/service/{}", cmd.address.address());
            Ok(())
        }
        Err(e) => Err(crate::error::Error::new(
            exitcode::CANTCREAT,
            miette!("An error occurred while creating secure channel listener").context(e),
        )),
    }
}

pub async fn create_listener(
    ctx: &Context,
    addr: Address,
    authorized_identifiers: Option<Vec<IdentityIdentifier>>,
    identity: Option<String>,
    mut base_route: Route,
) -> Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::create_secure_channel_listener(&addr, authorized_identifiers, identity)?,
        )
        .await?;

    let response = api::parse_create_secure_channel_listener_response(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            println!("/service/{}", addr.address());
            Ok(())
        }
        _ => {
            eprintln!("An error occurred while creating secure channel listener",);
            std::process::exit(exitcode::CANTCREAT)
        }
    }
}
