use crate::util::api::CloudOpts;
use crate::util::{api, node_rpc, RpcBuilder};
use crate::CommandGlobalOpts;
use crate::Result;
use clap::Args;
use miette::{miette, IntoDiagnostic};
use minicbor::{Decode, Decoder, Encode};
use ockam::identity::{Identifier, SecureChannelOptions, TrustIdentifierPolicy};
use ockam::{Context, Node, TcpConnectionOptions, TcpTransport};
use ockam_api::cli_state::identities::IdentityState;
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};
use ockam_api::cli_state::NodeState;
use ockam_api::nodes::models::base::NodeStatus as NodeStatusModel;
use ockam_api::nodes::NodeManager;
use ockam_core::api::{Request, Response, Status};
use ockam_core::route;
use ockam_node::MessageSendReceiveOptions;
use std::io::Write;
use std::time::Duration;

/// Display information about the system's status
#[derive(Clone, Debug, Args)]
pub struct StatusCommand {
    /// Show status for all identities; default: enrolled only
    #[arg(long, short)]
    all: bool,

    /// Override default timeout (in seconds)
    #[arg(long, default_value = "30")]
    timeout: u64,
}

impl StatusCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, StatusCommand)) -> miette::Result<()> {
    run_impl(ctx, opts, cmd).await
}

async fn run_impl(ctx: Context, opts: CommandGlobalOpts, cmd: StatusCommand) -> miette::Result<()> {
    let tcp = TcpTransport::create(&ctx).await.into_diagnostic()?;
    let identities_details = get_identities_details(&opts, cmd.all)?;
    let nodes_details = get_nodes_details(&ctx, &opts, &tcp).await?;
    let orchestrator_version =
        get_orchestrator_version(ctx, &opts, Duration::from_secs(cmd.timeout)).await;
    print_output(
        opts,
        orchestrator_version,
        identities_details,
        nodes_details,
    )?;
    Ok(())
}

async fn get_nodes_details(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    tcp: &TcpTransport,
) -> Result<Vec<NodeDetails>> {
    let mut node_details: Vec<NodeDetails> = vec![];

    let node_states = opts.state.nodes.list()?;
    if node_states.is_empty() {
        return Ok(node_details);
    }

    for node_state in &node_states {
        let node_infos = NodeDetails {
            identifier: node_state.config().identifier()?,
            state: node_state.clone(),
            status: get_node_status(ctx, opts, node_state, tcp).await?,
        };
        node_details.push(node_infos);
    }

    Ok(node_details)
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
        let resp = rpc.parse_response_body::<NodeStatusModel>()?;
        node_status = resp.status;
    }

    Ok(node_status)
}

fn get_identities_details(opts: &CommandGlobalOpts, all: bool) -> Result<Vec<IdentityState>> {
    let mut identities_details: Vec<IdentityState> = vec![];
    for identity in opts.state.identities.list()? {
        if all {
            identities_details.push(identity)
        } else {
            match &identity.config().enrollment_status {
                Some(_enrollment) => identities_details.push(identity),
                None => (),
            }
        }
    }
    Ok(identities_details)
}

async fn get_orchestrator_version(
    ctx: Context,
    opts: &CommandGlobalOpts,
    timeout: Duration,
) -> Result<OrchestratorVersionInfo> {
    let controller_addr = CloudOpts::route();
    let controller_tcp_addr = controller_addr.to_socket_addr()?;
    let controller_identifier = NodeManager::load_controller_identifier()?;
    let tcp = TcpTransport::create(&ctx).await?;
    let connection = tcp
        .connect(controller_tcp_addr, TcpConnectionOptions::new())
        .await?;
    let node = {
        let identities_vault = opts.state.vaults.default()?.get().await?;
        let identities_repository = opts.state.identities.identities_repository().await?;
        Node::builder()
            .with_identities_vault(identities_vault)
            .with_identities_repository(identities_repository)
            .build(ctx)
            .await?
    };
    let identifier = opts.state.identities.default()?.identifier();
    let secure_channel_options = SecureChannelOptions::new()
        .with_trust_policy(TrustIdentifierPolicy::new(controller_identifier))
        .with_timeout(timeout);
    let secure_channel = node
        .create_secure_channel(
            &identifier,
            route![connection, "api"],
            secure_channel_options,
        )
        .await?;

    // Send request
    let buf: Vec<u8> = node
        .send_and_receive_extended::<Vec<u8>>(
            route![secure_channel, "version_info"],
            Request::get("").to_vec()?,
            MessageSendReceiveOptions::new().with_timeout(timeout),
        )
        .await?
        .body();
    let mut dec = Decoder::new(&buf);
    let hdr = dec.decode::<Response>()?;
    if hdr.status() == Some(Status::Ok) {
        Ok(dec.decode::<OrchestratorVersionInfo>()?)
    } else {
        Err(miette!("Failed to retrieve version information from node.").into())
    }
}

fn print_output(
    opts: CommandGlobalOpts,
    orchestrator_version: Result<OrchestratorVersionInfo>,
    identities_details: Vec<IdentityState>,
    nodes_details: Vec<NodeDetails>,
) -> Result<()> {
    let status = StatusData::from_parts(orchestrator_version, identities_details, nodes_details)?;
    let plain = build_plain_output(&opts, &status)?;
    let json = serde_json::to_string(&status)?;
    opts.terminal
        .stdout()
        .plain(String::from_utf8(plain).expect("Invalid UTF-8 output"))
        .json(json)
        .write_line()?;
    Ok(())
}

fn build_plain_output(opts: &CommandGlobalOpts, status: &StatusData) -> Result<Vec<u8>> {
    let mut plain = Vec::new();
    writeln!(
        &mut plain,
        "Controller version: {}",
        status.orchestrator_version.controller_version
    )?;
    writeln!(
        &mut plain,
        "Project version: {}",
        status.orchestrator_version.project_version
    )?;
    if status.identities.is_empty() {
        writeln!(
            &mut plain,
            "No enrolled identities found! Try passing the `--all` argument to see all identities.",
        )?;
        return Ok(plain);
    }
    let default_identity = opts.state.identities.default()?;
    for (i_idx, i) in status.identities.iter().enumerate() {
        writeln!(&mut plain, "Identity[{i_idx}]")?;
        if default_identity.config().identifier() == i.identity.config().identifier() {
            writeln!(&mut plain, "{:2}Default: yes", "")?;
        }
        for line in i.identity.to_string().lines() {
            writeln!(&mut plain, "{:2}{}", "", line)?;
        }
        if !i.nodes.is_empty() {
            writeln!(&mut plain, "{:2}Linked Nodes:", "")?;
            for (n_idx, node) in i.nodes.iter().enumerate() {
                writeln!(&mut plain, "{:4}Node[{}]:", "", n_idx)?;
                writeln!(&mut plain, "{:6}Name: {}", "", node.name)?;
                writeln!(&mut plain, "{:6}Status: {}", "", node.status)?;
            }
        }
    }
    Ok(plain)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StatusData {
    #[serde(flatten)]
    orchestrator_version: OrchestratorVersionInfo,
    identities: Vec<IdentityWithLinkedNodes>,
}

impl StatusData {
    fn from_parts(
        orchestrator_version: Result<OrchestratorVersionInfo>,
        identities_details: Vec<IdentityState>,
        mut nodes_details: Vec<NodeDetails>,
    ) -> Result<Self> {
        let orchestrator_version = orchestrator_version.unwrap_or(OrchestratorVersionInfo {
            controller_version: "N/A".to_string(),
            project_version: "N/A".to_string(),
        });
        let mut identities = vec![];
        for identity in identities_details.into_iter() {
            let mut identity_status = IdentityWithLinkedNodes {
                identity,
                nodes: vec![],
            };
            nodes_details
                .retain(|nd| nd.identifier == identity_status.identity.config().identifier());
            if !nodes_details.is_empty() {
                for node in nodes_details.iter() {
                    identity_status.nodes.push(NodeStatus {
                        name: node.state.name().to_string(),
                        status: node.status.clone(),
                    });
                }
            }
            identities.push(identity_status);
        }
        Ok(Self {
            orchestrator_version,
            identities,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct IdentityWithLinkedNodes {
    identity: IdentityState,
    nodes: Vec<NodeStatus>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct IdentityStatus {}

#[derive(serde::Serialize, serde::Deserialize)]
struct NodeStatus {
    name: String,
    status: String,
}

struct NodeDetails {
    identifier: Identifier,
    state: NodeState,
    status: String,
}

#[derive(Encode, Decode, Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(test, derive(Clone))]
#[cbor(map)]
struct OrchestratorVersionInfo {
    #[cfg(feature = "tag")]
    #[n(0)]
    #[serde(skip)]
    tag: TypeTag<3783607>,
    #[n(1)]
    controller_version: String,
    #[n(2)]
    project_version: String,
}
