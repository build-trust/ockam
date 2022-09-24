use crate::util::{api, connect_to, exitcode, OckamConfig};
use crate::{help, node::HELP_DETAIL, CommandGlobalOpts};
use anyhow::Context;
use clap::Args;
use colorful::Colorful;
use ockam::Route;
use ockam_api::config::cli::NodeConfig;
use ockam_api::nodes::{models::base::NodeStatus, NODEMANAGER_ADDR};
use ockam_core::api::Status;
use std::time::Duration;

/// Show Nodes
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, help_template = help::template(HELP_DETAIL))]
pub struct ShowCommand {
    /// Name of the node.
    #[arg(default_value = "default")]
    node_name: String,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let cfg = &options.config;
        let port = match cfg.inner().nodes.get(&self.node_name) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };
        connect_to(
            port,
            (cfg.clone(), self.node_name, false),
            print_query_status,
        );
    }
}

// TODO: This function should be replaced with a better system of
// printing the node state in the future but for now we can just tell
// clippy to stop complainaing about it.
#[allow(clippy::too_many_arguments)]
fn print_node_info(node_cfg: &NodeConfig, node_name: &str, status: &str, default_id: &str) {
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
  Secure Channel Listener Address: /service/api
"#,
        node_name,
        match status {
            "UP" => status.light_green(),
            "DOWN" => status.light_red(),
            _ => status.white(),
        },
        node_cfg.port,
        node_cfg.port,
        default_id,
        default_id,
    );
}

pub async fn print_query_status(
    mut ctx: ockam::Context,
    (cfg, node_name, wait_until_ready): (OckamConfig, String, bool),
    mut base_route: Route,
) -> anyhow::Result<()> {
    let route = base_route.modify().append(NODEMANAGER_ADDR).into();
    let node_cfg = cfg.get_node(&node_name)?;

    // Wait until node is up.
    if query_status(&mut ctx, &route).await.is_err() {
        if wait_until_ready {
            let mut attempts = 10;
            while attempts > 0 {
                tokio::time::sleep(Duration::from_millis(250)).await;
                if query_status(&mut ctx, &route).await.is_ok() {
                    break;
                }
                attempts -= 1;
            }
            if attempts <= 0 {
                print_node_info(&node_cfg, &node_name, "DOWN", "N/A");
                return Ok(());
            }
        } else {
            print_node_info(&node_cfg, &node_name, "DOWN", "N/A");
            return Ok(());
        }
    }

    // Get short id for the node
    ctx.send(route.clone(), api::short_identity()?).await?;
    let resp = ctx
        .receive_duration_timeout::<Vec<u8>>(Duration::from_millis(250))
        .await
        .context("Failed to process request for short id")?;

    let (response, result) = api::parse_short_identity_response(&resp)?;
    let default_id = match response.status() {
        Some(Status::Ok) => {
            format!("{}", result.identity_id)
        }
        _ => String::from("NOT FOUND"),
    };

    print_node_info(&node_cfg, &node_name, "UP", &default_id);
    Ok(())
}

async fn query_status(ctx: &mut ockam::Context, route: &Route) -> anyhow::Result<()> {
    ctx.send(route.clone(), api::query_status()?).await?;

    let resp = ctx
        .receive_duration_timeout::<Vec<u8>>(Duration::from_millis(250))
        .await
        .context("Failed to process request");

    match resp {
        Ok(resp) => {
            let NodeStatus { .. } = api::parse_status(&resp)?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}
