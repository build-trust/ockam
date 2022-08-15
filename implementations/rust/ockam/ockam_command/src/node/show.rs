use crate::util::{self, api, connect_to, exitcode, OckamConfig};
use crate::CommandGlobalOpts;
use anyhow::Context;
use clap::Args;
use ockam::Route;
use ockam_api::config::cli::NodeConfig;
use ockam_api::nodes::{models::base::NodeStatus, NODEMANAGER_ADDR};
use ockam_api::Status;
use std::time::Duration;

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
                std::process::exit(exitcode::IOERR);
            }
        };
        connect_to(port, (cfg.clone(), command.node_name), query_status);
    }
}

// TODO: This function should be replaced with a better system of
// printing the node state in the future but for now we can just tell
// clippy to stop complaining about it.
#[allow(clippy::too_many_arguments)]
fn print_node_info(node_cfg: &NodeConfig, node_name: &str, status: &str, default_id: &str) {
    use util::dyn_info::{NodeService, ServiceType};
    let node_out = util::dyn_info::DynNodeInfo::new(node_name.to_string())
        .status(status.to_string())
        .service(NodeService::new(
            ServiceType::TCPListener,
            format!("/ip4/127.0.0.1/tcp/{}", node_cfg.port),
            None,
            None,
            None,
        ))
        .service(NodeService::new(
            ServiceType::SecureChannelListener,
            format!("/service/api"),
            Some(format!("/ip4/127.0.0.1/tcp/{}/service/api", node_cfg.port)),
            Some(default_id.to_string()),
            Some(vec![format!("{}", default_id)]),
        ))
        .service(NodeService::new(
            ServiceType::Uppercase,
            "/service/uppercase".to_string(),
            None,
            None,
            None,
        ))
        .service(NodeService::new(
            ServiceType::Echo,
            "/service/echo".to_string(),
            None,
            None,
            None,
        ))
        .secure_channel_addr_listener("/service/api".to_string());

    println!("{}", node_out);
}

pub async fn query_status(
    mut ctx: ockam::Context,
    (cfg, node_name): (OckamConfig, String),
    mut base_route: Route,
) -> anyhow::Result<()> {
    ctx.send(
        base_route.modify().append(NODEMANAGER_ADDR),
        api::query_status()?,
    )
    .await?;

    let resp = ctx
        .receive_duration_timeout::<Vec<u8>>(Duration::from_millis(333))
        .await
        .context("Failed to process request");

    let node_cfg = cfg.get_node(&node_name).unwrap();

    match resp {
        Ok(resp) => {
            let NodeStatus { .. } = api::parse_status(&resp)?;

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

            print_node_info(&node_cfg, &node_name, "UP", &default_id)
        }
        Err(_) => print_node_info(&node_cfg, &node_name, "DOWN", "N/A"),
    }

    util::stop_node(ctx).await
}
