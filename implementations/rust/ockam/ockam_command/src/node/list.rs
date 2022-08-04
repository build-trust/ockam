use std::time::Duration;

use crate::util::{self, api, connect_to};
use crate::{node::show::query_status, CommandGlobalOpts, OckamConfig};
use clap::Args;
use crossbeam_channel::{bounded, Sender};
use ockam::{Context, Route};
use ockam_api::nodes::NODEMANAGER_ADDR;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(opts: CommandGlobalOpts, _: ListCommand) {
        let cfg = &opts.config;
        let node_names = {
            let inner = cfg.get_inner();

            if inner.nodes.is_empty() {
                println!("No nodes registered on this system!");
                std::process::exit(0);
            }

            // Before printing node state we have to verify it.  This
            // happens by sending a QueryStatus request to every node on
            // record.  If the function fails, then it is assumed not to
            // be up.  Also, if the function returns, but yields a
            // different pid, then we update the pid stored in the config.
            // This should only happen if the node has failed in the past,
            // and has been restarted by something that is not this CLI.
            inner.nodes.iter().map(|(name, _)| name.clone()).collect()
        };
        verify_pids(cfg, node_names);

        cfg.get_inner()
            .nodes
            .iter()
            .for_each(|(node_name, node_cfg)| {
                connect_to(
                    node_cfg.port,
                    (cfg.clone(), node_name.clone()),
                    query_status,
                )
            });
    }
}

// TODO: move to utils directory
fn verify_pids(cfg: &OckamConfig, nodes: Vec<String>) {
    for node_name in nodes {
        let node_cfg = cfg.get_node(&node_name).unwrap();

        let (tx, rx) = bounded(1);

        connect_to(node_cfg.port, tx, query_pid);
        let verified_pid = rx.recv().unwrap();

        if node_cfg.pid != verified_pid {
            if let Err(e) = cfg.update_pid(&node_name, verified_pid) {
                eprintln!("failed to update pid for node {}: {}", node_name, e);
            }
        }
    }
}

pub async fn query_pid(
    mut ctx: Context,
    tx: Sender<Option<i32>>,
    mut base_route: Route,
) -> anyhow::Result<()> {
    ctx.send(
        base_route.modify().append(NODEMANAGER_ADDR),
        api::query_status()?,
    )
    .await?;

    let resp = match ctx
        .receive_duration_timeout::<Vec<u8>>(Duration::from_millis(200))
        .await
    {
        Ok(r) => r.take().body(),
        Err(_) => {
            tx.send(None).unwrap();
            return util::stop_node(ctx).await;
        }
    };

    let status = api::parse_status(&resp)?;
    tx.send(Some(status.pid)).unwrap();
    util::stop_node(ctx).await
}
