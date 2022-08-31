use crate::secure_channel::HELP_DETAIL;
use crate::util::{api, connect_to, exitcode, get_final_element};
use crate::{help, CommandGlobalOpts};

use clap::Args;

use ockam::identity::IdentityIdentifier;

use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::Status;
use ockam_core::{Address, Route};

/// Create Secure Channel Listeners
#[derive(Clone, Debug, Args)]
#[clap(arg_required_else_help = true, help_template = help::template(HELP_DETAIL))]
pub struct CreateCommand {
    #[clap(flatten)]
    node_opts: SecureChannelListenerNodeOpts,

    /// Address for this listener
    address: Address,

    /// Authorized Identifiers of secure channel initiators
    #[clap(short, long, value_name = "IDENTIFIER")]
    authorized_identifier: Option<Vec<IdentityIdentifier>>,
}

#[derive(Clone, Debug, Args)]
pub struct SecureChannelListenerNodeOpts {
    /// Node at which to create the listener
    #[clap(global = true, long, value_name = "NODE", default_value = "default")]
    pub at: String,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let cfg = options.config;
        let node = get_final_element(&self.node_opts.at);
        let port = match cfg.select_node(node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };

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
