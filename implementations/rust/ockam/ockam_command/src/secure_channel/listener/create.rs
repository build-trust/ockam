use anyhow::anyhow;
use clap::Args;

use ockam::identity::IdentityIdentifier;
use ockam::Context;
use ockam_api::nodes::models::secure_channel::CreateSecureChannelListenerRequest;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{Request, Status};
use ockam_core::{Address, Route};

use crate::secure_channel::HELP_DETAIL;
use crate::util::{api, exitcode, extract_address_value, node_rpc, Rpc};
use crate::{help, CommandGlobalOpts};

/// Create Secure Channel Listeners
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = help::template(HELP_DETAIL))]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: SecureChannelListenerNodeOpts,

    /// Address for this listener
    address: Address,

    /// Authorized Identifiers of secure channel initiators
    #[arg(short, long, value_name = "IDENTIFIERS")]
    authorized_identifiers: Option<Vec<IdentityIdentifier>>,
}

#[derive(Clone, Debug, Args)]
pub struct SecureChannelListenerNodeOpts {
    /// Node at which to create the listener
    #[arg(global = true, long, value_name = "NODE", default_value = "default")]
    pub at: String,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
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
    let node = extract_address_value(&cmd.node_opts.at)?;
    let mut rpc = Rpc::background(ctx, &opts, &node)?;
    let req = Request::post("/node/secure_channel_listener").body(
        CreateSecureChannelListenerRequest::new(&cmd.address, cmd.authorized_identifiers),
    );
    rpc.request(req).await?;
    match rpc.is_ok() {
        Ok(_) => {
            println!("/service/{}", cmd.address.address());
            Ok(())
        }
        Err(e) => Err(crate::error::Error::new(
            exitcode::CANTCREAT,
            anyhow!("An error occurred while creating secure channel listener").context(e),
        )),
    }
}

pub async fn create_listener(
    ctx: &Context,
    addr: Address,
    authorized_identifiers: Option<Vec<IdentityIdentifier>>,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::create_secure_channel_listener(&addr, authorized_identifiers)?,
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
