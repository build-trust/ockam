use crate::util::{self, api, connect_to, OckamConfig};
use crate::CommandGlobalOpts;
use anyhow::Context;
use clap::Args;
use ockam::Route;
use ockam_api::nodes::{models::base::NodeStatus, NODEMANAGER_ADDR};
use ockam_api::Status;
use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

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
        node_name,
        status,
        workers,
        pid,
        transports,
        ..
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
    let api_address = SocketAddr::new(IpAddr::from_str("127.0.0.1").unwrap(), node_cfg.port);
    let (mlog, _) = cfg.log_paths_for_node(&node_name.to_string()).unwrap();
    let log_path = util::print_path(&mlog);

    println!(
        r#"
Node:
  Name: {}
  Status: {}
  API Address: {}
  Default Identity: {}
  Pid: {}
  Worker count: {}
  Transport count: {}
  Log Path: {}
"#,
        node_name, status, api_address, default_id, pid, workers, transports, log_path
    );
    util::stop_node(ctx).await
}
