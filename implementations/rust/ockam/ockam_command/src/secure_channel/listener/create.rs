use crate::secure_channel::HELP_DETAIL;
use crate::util::{api, connect_to, exitcode, extract_node_name};
use crate::{help, CommandGlobalOpts};

use clap::Args;

use ockam::identity::IdentityIdentifier;

use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::Status;
use ockam_core::{Address, Route};

/// Create Secure Channel Listeners
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, help_template = help::template(HELP_DETAIL))]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: SecureChannelListenerNodeOpts,

    /// Address for this listener
    address: Address,

    /// Authorized Identifiers of secure channel initiators
    #[arg(short, long, value_name = "IDENTIFIER")]
    authorized_identifier: Option<Vec<IdentityIdentifier>>,
}

#[derive(Clone, Debug, Args)]
pub struct SecureChannelListenerNodeOpts {
    /// Node at which to create the listener
    #[arg(global = true, long, value_name = "NODE", default_value = "default")]
    pub at: String,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let cfg = options.config;
        let node = extract_node_name(&self.node_opts.at).unwrap_or_else(|_| "".to_string());
        let port = cfg.get_node_port(&node).unwrap();

        connect_to(port, self, |ctx, cmd, rte| async {
            create_listener(&ctx, cmd.address, cmd.authorized_identifier, rte).await?;
            drop(ctx);
            Ok(())
        });
    }
}

pub async fn create_listener(
    ctx: &ockam::Context,
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
