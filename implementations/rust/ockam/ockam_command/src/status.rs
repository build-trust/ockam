use crate::util::{api, node_rpc, RpcBuilder};
use crate::CommandGlobalOpts;
use crate::Result;
use anyhow::anyhow;
use clap::Args;
use ockam::{Context, TcpTransport};
use ockam_api::cli_state::identities::IdentityState;
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};
use ockam_api::cli_state::NodeState;
use ockam_api::nodes::models::base::NodeStatus;
use ockam_identity::IdentityIdentifier;
use std::time::Duration;

/// Display Ockam Status
#[derive(Clone, Debug, Args)]
pub struct StatusCommand {
    /// Show status for all identities, default: enrolled only
    #[arg(long, short)]
    all: bool,
}

struct NodeDetails {
    identifier: IdentityIdentifier,
    state: NodeState,
    status: String,
}

impl StatusCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, StatusCommand)) -> Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: StatusCommand) -> Result<()> {
    let node_states = opts.state.nodes.list()?;
    if node_states.is_empty() {
        return Err(anyhow!("No nodes registered on this system!").into());
    }

    let mut node_details: Vec<NodeDetails> = vec![];
    let tcp = TcpTransport::create(ctx).await?;
    for node_state in &node_states {
        let node_infos = NodeDetails {
            identifier: node_state.config().identifier().await?,
            state: node_state.clone(),
            status: get_node_status(ctx, &opts, node_state, &tcp).await?,
        };
        node_details.push(node_infos);
    }

    let mut status_identities: Vec<IdentityState> = vec![];
    for identity in opts.state.identities.list()? {
        if cmd.all {
            status_identities.push(identity)
        } else {
            match &identity.config().enrollment_status {
                Some(_enrollment) => status_identities.push(identity),
                None => (),
            }
        }
    }

    print_status(&opts, status_identities, node_details).await?;

    Ok(())
}

async fn get_node_status(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_state: &NodeState,
    tcp: &TcpTransport,
) -> Result<String> {
    let mut node_status: String = "Stopped".to_string();
    let mut rpc = RpcBuilder::new(ctx, opts, node_state.name())
        .tcp(tcp)?
        .build();
    if rpc
        .request_with_timeout(api::query_status(), Duration::from_millis(200))
        .await
        .is_ok()
    {
        let resp = rpc.parse_response::<NodeStatus>()?;
        node_status = resp.status.to_string();
    }

    Ok(node_status)
}

async fn print_status(
    opts: &CommandGlobalOpts,
    identities: Vec<IdentityState>,
    mut node_details: Vec<NodeDetails>,
) -> Result<()> {
    if identities.is_empty() {
        return Err(anyhow!(
            "No enrolled identities found! Try passing the `--all` argument to see all identities."
        )
        .into());
    }
    let default_identity = opts.state.identities.default()?;

    for (i_idx, identity) in identities.iter().enumerate() {
        println!("Identity[{i_idx}]");
        if default_identity.config().identifier() == identity.config().identifier() {
            println!("{:2}Default: yes", "")
        }
        for line in identity.to_string().lines() {
            println!("{:2}{}", "", line);
        }

        node_details.retain(|nd| nd.identifier == identity.config().identifier());
        if !node_details.is_empty() {
            println!("{:2}Linked Nodes:", "");
            for (n_idx, node) in node_details.iter().enumerate() {
                println!("{:4}Node[{}]:", "", n_idx);
                println!("{:6}Name: {}", "", node.state.name());
                println!("{:6}Status: {}", "", node.status)
            }
        }
    }
    Ok(())
}
