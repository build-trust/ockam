use crate::util::{self, api, connect_to, OckamConfig};
use crate::CommandGlobalOpts;
use anyhow::Context;
use clap::Args;
use ockam::Route;
use ockam_api::nodes::{models::base::NodeStatus, NODEMANAGER_ADDR};
use ockam_api::Status;

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Name of the node.
    #[clap(default_value = "default")]
    node_name: String,
}

impl ShowCommand {
    pub fn run(opts: CommandGlobalOpts, command: ShowCommand) {
        let cfg = &opts.config;
        let port = match cfg.get_inner().nodes.get(&command.node_name) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };
        connect_to(port, cfg.clone(), query_status);
    }
}

pub async fn query_status(
    ctx: ockam::Context,
    cfg: OckamConfig,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::query_status()?,
        )
        .await
        .context("Failed to process request")?;

    let NodeStatus {
        node_name, status, ..
    } = api::parse_status(&resp)?;

    // Getting short id for the node
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::short_identity()?,
        )
        .await
        .context("Failed to process request for short id")?;

    let (response, result) = api::parse_short_identity_response(&resp)?;
    let default_id = match response.status() {
        Some(Status::Ok) => {
            format!("{}", result.identity_id)
        }
        _ => String::from("NOT FOUND"),
    };

    let node_cfg = cfg.get_node(&node_name).unwrap();

    println!(
        r#"
Node:
  Name: {}
  Status: {}
  Services:
    Service:
      Type: TCP Listener
      Address: /ip4/127.0.0.1/tcp/{}
    Service:
      Type: Secure Channel Listener
      Address: /service/api
      Route: /ip4/127.0.0.1/tcp/{}/service/api
      Identity: {}
      Authorized Identities:
        - {}
    Service:
      Type: Uppercase
      Address: /service/uppercase
    Service:
      Type: Echo
      Address: /service/echo
"#,
        node_name, status, node_cfg.port, node_cfg.port, default_id, default_id,
    );
    util::stop_node(ctx).await
}
