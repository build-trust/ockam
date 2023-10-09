use std::fmt::Debug;
use std::io::Write;
use std::time::Duration;

use clap::Args;
use miette::miette;
use minicbor::{Decode, Decoder, Encode};
use tracing::warn;

use ockam::identity::{Identifier, SecureChannelOptions, TrustIdentifierPolicy};
use ockam::{Context, Node, TcpConnectionOptions, TcpTransport};
use ockam_api::cli_state::identities::IdentityState;
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};
use ockam_api::cli_state::NodeState;
use ockam_api::nodes::models::base::NodeStatus as NodeStatusModel;
use ockam_api::nodes::{BackgroundNode, NodeManager};
use ockam_core::api::{Request, ResponseHeader, Status};
use ockam_core::route;
use ockam_node::MessageSendReceiveOptions;

use crate::error::Error;
use crate::util::{api, node_rpc};
use crate::CommandGlobalOpts;
use crate::Result;

const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

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
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: StatusCommand,
) -> miette::Result<()> {
    let identities_details = get_identities_details(&opts, cmd.all)?;
    let nodes_details = get_nodes_details(ctx, &opts).await?;
    let orchestrator_version =
        get_orchestrator_version(ctx, &opts, Duration::from_secs(cmd.timeout)).await;
    let status = StatusData::from_parts(orchestrator_version, identities_details, nodes_details)?;
    print_output(opts, cmd, status)?;
    Ok(())
}

async fn get_nodes_details(ctx: &Context, opts: &CommandGlobalOpts) -> Result<Vec<NodeDetails>> {
    let mut node_details: Vec<NodeDetails> = vec![];

    let node_states = opts.state.nodes.list()?;
    if node_states.is_empty() {
        return Ok(node_details);
    }
    let default_node_name = opts.state.nodes.default()?.name().to_string();
    let mut node = BackgroundNode::create(ctx, &opts.state, &default_node_name).await?;
    node.set_timeout(Duration::from_millis(200));

    for node_state in &node_states {
        node.set_node_name(node_state.name());
        let node_infos = NodeDetails {
            identifier: node_state.config().identifier()?,
            state: node_state.clone(),
            status: get_node_status(ctx, &node).await?,
        };
        node_details.push(node_infos);
    }

    Ok(node_details)
}

async fn get_node_status(ctx: &Context, node: &BackgroundNode) -> Result<String> {
    let node_status_model: miette::Result<NodeStatusModel> =
        node.ask(ctx, api::query_status()).await;
    Ok(node_status_model
        .map(|m| m.status)
        .unwrap_or("Stopped".to_string()))
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
    ctx: &Context,
    opts: &CommandGlobalOpts,
    timeout: Duration,
) -> Result<OrchestratorVersionInfo> {
    // for new we get the controller address directly until we
    // access a Controller interface from the NodeManager
    let controller_addr = NodeManager::controller_multiaddr();
    let controller_identifier = NodeManager::load_controller_identifier()?;
    let controller_tcp_addr = controller_addr.to_socket_addr()?;
    let tcp = TcpTransport::create(ctx).await?;
    let connection = tcp
        .connect(controller_tcp_addr, TcpConnectionOptions::new())
        .await?;

    // Create node that will be used to send the request
    let node = {
        // Get or create a vault to store the identity
        let vault = match opts.state.vaults.default() {
            Ok(v) => v,
            Err(_) => opts.state.create_vault_state(None).await?,
        }
        .get()
        .await?;
        let identities_repository = opts.state.identities.identities_repository().await?;
        Node::builder()
            .with_vault(vault)
            .with_identities_repository(identities_repository)
            .build(ctx)
            .await?
    };

    // Establish secure channel with controller
    let node_identifier = opts
        .state
        .default_identities()
        .await?
        .identities_creation()
        .create_identity()
        .await?;
    let secure_channel_options = SecureChannelOptions::new()
        .with_trust_policy(TrustIdentifierPolicy::new(controller_identifier))
        .with_timeout(timeout);
    let secure_channel = node
        .create_secure_channel(
            &node_identifier,
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

    // Decode response
    let hdr = dec.decode::<ResponseHeader>()?;
    if hdr.status() == Some(Status::Ok) {
        Ok(dec.decode::<OrchestratorVersionInfo>()?)
    } else {
        Err(miette!("Failed to retrieve version information from node.").into())
    }
}

fn print_output(opts: CommandGlobalOpts, cmd: StatusCommand, status: StatusData) -> Result<()> {
    let plain = build_plain_output(&opts, &cmd, &status)?;
    let json = serde_json::to_string(&status)?;
    opts.terminal
        .stdout()
        .plain(String::from_utf8(plain).expect("Invalid UTF-8 output"))
        .json(json)
        .write_line()?;
    Ok(())
}

fn confirm_emoji(condition: bool) -> String {
    if condition {
        "✔️️".to_string()
    } else {
        "✖️".to_string()
    }
}

fn linked_nodes_with_status(nodes: &[NodeStatus]) -> String {
    nodes
        .iter()
        .map(|node| {
            let emoji = match node.status.as_str() {
                "Running" => "🟢",
                "Stopped" => "🔴",
                _ => "❓",
            };
            format!("       - {} {} ({})", emoji, node.name, node.status)
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn return_if_no_identities_or_no_enrolled(
    status: &StatusData,
    cmd: &StatusCommand,
    mut plain: Vec<u8>,
) -> Result<Vec<u8>> {
    if status.identities.is_empty() {
        if cmd.all {
            writeln!(&mut plain, "No identities found!")?;
        } else {
            writeln!(
                &mut plain,
                "No enrolled identities could be found. \
                Try passing the `--all` argument to see all identities, and not just the enrolled ones. \
                Also consider running `ockam enroll` to enroll an identity.",
            )?;
        }
        return Ok(plain);
    }
    Ok(plain)
}

fn build_plain_output(
    opts: &CommandGlobalOpts,
    cmd: &StatusCommand,
    status: &StatusData,
) -> Result<Vec<u8>> {
    let mut plain = return_if_no_identities_or_no_enrolled(status, cmd, Vec::new())?;
    let default_identity = opts.state.identities.default()?;

    let has_credentials = {
        if opts.state.credentials.default().is_ok() {
            let default_credentials = opts.state.credentials.default().unwrap();
            let default_credential_identifier = &default_credentials.config().issuer_identifier;
            default_credential_identifier == &default_identity.identifier()
        } else {
            false
        }
    };

    let Ok(default_user) = opts.state.users_info.default() else {
        return Err(Error::InternalError {error_message: "Default user is required".to_string(), exit_code: 0})
    };

    // --- INFORMATION TO PRINT ---

    let header = opts.terminal.build_header("Status", String::new())?;

    // NOTE: I'm not sure there is any way to know the email associated
    // with an enrollment, this is only the email of the default user
    let default_user_email = &default_user.config().email;
    let trust_context_name = opts
        .state
        .trust_contexts
        .default()
        .map_or("No trust context".to_string(), |context| {
            context.name().to_string()
        });
    let default_identity_name = default_identity.name();
    let is_enrolled_emoji = confirm_emoji(default_identity.is_enrolled());
    let has_credential_emoji = confirm_emoji(has_credentials);

    let Some(identity) = status
        .identities
        .iter()
        .find(|identity| identity.identity.name() == default_identity_name) else {
        return Err(Error::InternalError {error_message: "".to_string(), exit_code: 0})
    };

    let linked_nodes = linked_nodes_with_status(&identity.nodes);

    // --- INFORMATION TO PRINT ---

    writeln!(&mut plain, "{header}")?;
    writeln!(&mut plain, "Ockam Command V{}", PKG_VERSION)?;
    writeln!(&mut plain, "Default User Email: {default_user_email}")?;
    writeln!(&mut plain, "Trust Context: {trust_context_name}")?;
    writeln!(
        &mut plain,
        r#"Default Identity Information:
    Name: {default_identity_name}
    Is enrolled: {is_enrolled_emoji}
    Has credential: {has_credential_emoji}
    Linked nodes:
{linked_nodes}"#
    )?;
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
        let orchestrator_version = orchestrator_version
            .map_err(|e| warn!(%e, "Failed to retrieve orchestrator version"))
            .unwrap_or(OrchestratorVersionInfo {
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

#[derive(serde::Serialize, serde::Deserialize, Debug)]
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
    #[n(1)]
    controller_version: String,
    #[n(2)]
    project_version: String,
}
