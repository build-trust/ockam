use std::net::SocketAddr;
use std::str::FromStr;

use anyhow::Context;
use tracing::{debug, trace};

use ockam::TcpTransport;
use ockam_api::config::cli::NodeConfig;
use ockam_api::nodes::models::base::NodeStatus;

use crate::node;
use crate::util::{api, RpcBuilder};
use crate::CommandGlobalOpts;

use super::*;

pub async fn default_node(
    ctx: &ockam::Context,
    opts: &CommandGlobalOpts,
    tcp: &TcpTransport,
) -> Result<NodeConfig> {
    let no_nodes = {
        let cfg = opts.config.get_inner();
        cfg.nodes.is_empty()
    };

    // If there are no spawned nodes, create one called "default" and return it.
    let node = if no_nodes {
        debug!("No nodes found in config, creating default node");
        create_node(ctx, opts, "default").await?
    }
    // If there are spawned nodes, return the "default" node if exists and it's running
    // or the first node we find that is running.
    else {
        let node_names = {
            let cfg = opts.config.get_inner();
            cfg.nodes
                .iter()
                .map(|(name, _)| name.to_string())
                .collect::<Vec<_>>()
        };
        // Find all running nodes, skip those that are stopped.
        let mut ncs = vec![];
        for node_name in node_names.iter() {
            trace!(%node_name, "Checking node");
            let nc = opts.config.get_node(node_name)?;
            let mut rpc = RpcBuilder::new(ctx, opts, node_name).tcp(tcp).build()?;
            if rpc
                .request_with_timeout(
                    api::node::query_status(),
                    core::time::Duration::from_millis(333),
                )
                .await
                .is_err()
            {
                trace!(%node_name, "Node is not running");
                continue;
            }
            let ns = rpc.parse_response::<NodeStatus>()?;
            // Update PID if changed
            if nc.pid != Some(ns.pid) {
                opts.config.update_pid(&ns.node_name, ns.pid)?;
            }
            ncs.push(nc);
        }
        // Persist PID config changes
        opts.config.atomic_update().run()?;
        // No running nodes, create a new one
        if ncs.is_empty() {
            debug!("All existing nodes are stopped, creating a new one with a random name");
            create_node(ctx, opts, None).await?
        }
        // Return the "default" node or the first one of the list
        else {
            match ncs.iter().find(|ns| ns.name == "default") {
                None => ncs
                    .drain(..1)
                    .next()
                    .expect("already checked that is not empty"),
                Some(n) => n.clone(),
            }
        }
    };
    debug!("Using `{}` as the default node", node.name);
    Ok(node)
}

async fn create_node(
    ctx: &ockam::Context,
    opts: &CommandGlobalOpts,
    name: impl Into<Option<&'static str>>,
) -> Result<NodeConfig> {
    let node_name = name
        .into()
        .map(|name| name.to_string())
        .unwrap_or_else(node::random_name);
    match opts.config.select_node(&node_name) {
        Some(node) => {
            debug!(%node_name, "Returning existing node");
            Ok(node)
        }
        None => {
            debug!(%node_name, "Creating node");
            let cmd = node::CreateCommand {
                node_name: node_name.clone(),
                foreground: false,
                tcp_listener_address: "127.0.0.1:0".to_string(),
                skip_defaults: false,
                child_process: false, // this value is ignored in this case
                launch_config: None,
                no_watchdog: false,
            };
            let cmd = cmd.overwrite_addr()?;
            let addr = SocketAddr::from_str(&cmd.tcp_listener_address)
                .context("Failed to parse tcp listener address")?;
            let child_ctx = ctx.new_detached(Address::random_local()).await?;
            node::CreateCommand::create_background_node(child_ctx, (opts.clone(), cmd, addr))
                .await?;
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if let Some(node) = opts.config.select_node(&node_name) {
                    return Ok(node);
                }
            }
        }
    }
}
