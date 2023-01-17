use crate::util::{api, node_rpc, RpcBuilder};
use crate::Result;
use crate::{exitcode, CommandGlobalOpts};
use anyhow::anyhow;
use clap::Args;
use ockam::{Context, TcpTransport};
use ockam_api::cli_state::{IdentityState, NodeState};
use ockam_api::lmdb::LmdbStorage;
use ockam_api::nodes::models::base::NodeStatus;
use ockam_identity::Identity;
use ockam_vault::Vault;
use std::time::Duration;

/// Display Ockam status
#[derive(Clone, Debug, Args)]
pub struct StatusCommand {
    /// Show status for all identities, default: enrolled only
    #[arg(long, short)]
    all: bool,
}

struct NodeDetails {
    identity: Identity<Vault, LmdbStorage>,
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
    let node_states = match opts.state.nodes.list() {
        Ok(nodes) => nodes,
        Err(_err) => {
            return Err(crate::Error::new(
                exitcode::IOERR,
                anyhow!("No nodes registered on this system!"),
            ));
        }
    };

    let mut node_details: Vec<NodeDetails> = vec![];
    for node_state in &node_states {
        let node_infos = NodeDetails {
            identity: node_state
                .config
                .identity(
                    ctx,
                    /* FIXME: @adrian */ &LmdbStorage::new("wrong/path").await?,
                )
                .await?,
            state: node_state.clone(),
            status: get_node_status(ctx, &opts, node_state).await?,
        };
        node_details.push(node_infos);
    }

    let mut status_identities: Vec<IdentityState> = vec![];
    for identity in opts.state.identities.list()? {
        if cmd.all {
            status_identities.push(identity)
        } else {
            match &identity.config.enrollment_status {
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
) -> Result<String> {
    let mut node_status: String = "Stopped".to_string();

    let tcp = TcpTransport::create(ctx).await?;
    let mut rpc = RpcBuilder::new(ctx, opts, &node_state.config.name)
        .tcp(&tcp)?
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
    let default_identity = opts.state.identities.default()?;

    for (i_idx, identity) in identities.iter().enumerate() {
        println!("Identity[{}]", i_idx);
        if default_identity.config.identifier == identity.config.identifier {
            println!("{:2}Default: yes", "")
        }
        for line in identity.to_string().lines() {
            println!("{:2}{}", "", line);
        }

        node_details.retain(|nd| nd.identity.identifier() == &identity.config.identifier);
        if !node_details.is_empty() {
            println!("{:2}Linked Nodes:", "");
            for (n_idx, node) in node_details.iter().enumerate() {
                println!("{:4}Node[{}]:", "", n_idx);
                println!("{:6}Name: {}", "", node.state.config.name);
                println!("{:6}Status: {}", "", node.status)
            }
        }
    }
    Ok(())
}
